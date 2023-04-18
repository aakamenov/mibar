# mibar
Custom status bar for Wayland.

## Motivation
I'm tired of stitching together status bars using random shell scripts, config formats or languages that you have to guess how they work.
Also doing anything more advanced always ends up being hacky, needlessly complex and inefficient. I want to be able to easily extend or
change whatever I like and the result to be a single binary. I want code as configuration.

## How
We talk to Wayland using the [smithay-client-toolkit](https://crates.io/crates/smithay-client-toolkit) and draw on the CPU with [tiny-skia](https://crates.io/crates/tiny-skia)
using a custom retained widget system which is relatively simple but powerful and flexible enough for our needs. It should be easy to customize or extend the bar with very
little code by taking advantage of the specialized widgets built on top of the widget system. Eventually, it might evolve into a library that anyone can use to build their
bar as they desire, but the initial goal is for the code to only implement the functionality that I need.
