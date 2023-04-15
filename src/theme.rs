use tiny_skia::Color;

pub struct Theme {
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,
    pub muted: Color,
    pub subtle: Color,
    pub text: Color,
    pub warm1: Color,
    pub warm2: Color,
    pub warm3: Color,
    pub cold1: Color,
    pub cold2: Color,
    pub cold3: Color
}

impl Theme {
    #[inline]
    pub fn light() -> Self {
        Self {
            base: Color::from_rgba8(250, 244, 237, 255),
            surface: Color::from_rgba8(255, 250, 243, 255),
            overlay: Color::from_rgba8(242, 233, 222, 255),
            muted: Color::from_rgba8(152, 147, 165, 255),
            subtle: Color::from_rgba8(121, 117, 147, 255),
            text: Color::from_rgba8(87, 82, 121, 255),
            warm1: Color::from_rgba8(180, 99, 122, 255),
            warm2: Color::from_rgba8(234, 157, 52, 255),
            warm3: Color::from_rgba8(215, 130, 126, 255),
            cold1: Color::from_rgba8(40, 105, 131, 255),
            cold2: Color::from_rgba8(86, 148, 159, 255),
            cold3: Color::from_rgba8(144, 122, 169, 255)
        }
    }
}
