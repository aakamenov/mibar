use std::{
    env::{self, VarError},
    any::Any
};

use tiny_skia::Color;
use tokio::{
    net::UnixStream,
    io::AsyncReadExt
};

use crate::{
    geometry::{Size, Circle},
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event, ValueSender
    }
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

const WORKSPACE_COUNT: usize = 8;
const RADIUS: f32 = 8f32;
const SPACING: f32 = 3f32;

type WorkspaceNum = u8;

pub struct Workspaces {
    radius: f32
}

impl Workspaces {
    pub fn new() -> Self {
        Self { radius: RADIUS }
    }
}

impl Widget for Workspaces {
    fn init(&mut self, ctx: &mut InitCtx) {
        let Some(socket) = hyprland_socket() else {
            return;
        };
        
        ctx.task_with_sender(|sender: ValueSender<WorkspaceNum>| {
            async move {
                let mut stream = match UnixStream::connect(socket).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        eprintln!("Error opening Hyprland socket: {err}");
                        return;
                    }
                };

                let mut retries = 0;

                loop {
                    let mut buf = [0; 1024];

                    let read = match stream.read(&mut buf).await {
                        Ok(read) => {
                            retries = 0;

                            read
                        },
                        Err(err) => {
                            retries += 1;
                            eprintln!("Error while reading from Hyprland socket: {err}");
                            
                            if retries == 3 {
                                eprintln!("Closing listener...");
                                break;
                            }

                            continue;
                        }
                    };

                    let text = unsafe {
                        std::str::from_utf8_unchecked(&buf[..read])
                    };

                    const WORKSPACE: &str = "workspace>>";

                    if let Some(index) = text.find(WORKSPACE) {
                        let (_, num_start) = text.split_at(index + WORKSPACE.len());

                        // There should always be a new line.
                        let Some(new_line) = num_start.find('\n') else {
                            eprintln!("Missing new line in hyprland output.");

                            continue;
                        };

                        let (num, _) = num_start.split_at(new_line);
                        match num.parse() {
                            Ok(workspace) => sender.send(workspace).await,
                            Err(_) => eprintln!("Error parsing workspace number. Hyprland output:\n{text}")
                        }
                    }
                }
            }
        });
    }

    fn event(&mut self, ctx: &mut UpdateCtx, event: &Event) { }

    fn task_result(&mut self, _ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        let workspace = data.downcast_ref::<WorkspaceNum>().unwrap();
        println!("Changed to workspace: {workspace}");
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let diameter = self.radius * 2f32;
        let diameter = diameter.clamp(bounds.min.height, bounds.max.height);
        self.radius = diameter / 2f32;

        let count = WORKSPACE_COUNT as f32;
        let spacing = (SPACING * count) - 1f32;
        let width = (diameter * count) + spacing;

        let size = bounds.constrain(Size {
            width,
            height: diameter
        });

        size
    }

    fn draw(&mut self, ctx: &mut DrawCtx) {
        let layout = ctx.layout();
        let y = layout.y + self.radius;
        let mut x = layout.x + self.radius;

        for _ in 0..WORKSPACE_COUNT {
            let circle = Circle { x, y, radius: self.radius };
            ctx.fill_circle(circle, Color::BLACK);
            
            x += (self.radius * 2f32) + SPACING;
        }
    }
}

fn hyprland_socket() -> Option<String> {
    const ENV_VAR: &str = "HYPRLAND_INSTANCE_SIGNATURE";

    match env::var(ENV_VAR) {
        Ok(var) => {
            let path = format!("/tmp/hypr/{var}/.socket2.sock");

            Some(path)
        },
        Err(VarError::NotPresent) => {
            eprintln!("Hyprland envrionment variable ({ENV_VAR}) not present.");

            None
        }
        Err(VarError::NotUnicode(_)) => {
            eprintln!("Hyprland envrionment variable ({ENV_VAR}) is present but is not valid unicode.");

            None
        }
    }
}
