use clap::Parser;
use fs_tree::{render_tree::RenderTree, scan_tree::scan_dir};
use gpui::{
    canvas, div, prelude::*, quad, rgb, rgba, App, Application, Bounds, Pixels, Point, Size,
    Window, WindowOptions,
};
use std::{path::PathBuf, sync::Arc};

fn render_children(
    tree: &RenderTree,
    horizontal: bool,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    match tree {
        RenderTree::File {
            color,
            x,
            y,
            dx,
            dy,
            ..
        } => {
            let px = bounds.origin.x.0 + x * bounds.size.width.0;
            let py = bounds.origin.y.0 + y * bounds.size.height.0;
            let pdx = dx * bounds.size.width.0;
            let pdy = dy * bounds.size.height.0;

            window.paint_quad(quad(
                Bounds {
                    origin: Point::new(Pixels(px), Pixels(py)),
                    size: Size::new(Pixels(pdx), Pixels(pdy)),
                },
                0.0,
                rgb(*color),
                0.0,
                rgba(0),
                gpui::BorderStyle::Solid,
            ));
        }
        RenderTree::Dir { children, .. } => {
            for child in children {
                render_children(child, horizontal, bounds, window, cx);
            }
        }
    }
}

struct Program(Arc<RenderTree>);

impl Render for Program {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let children = Arc::clone(&self.0);
        let canvas = canvas(
            |_bounds, _window, _cx| (),
            move |bounds, _prepaint_data, window, cx| {
                render_children(&children, false, bounds, window, cx);
            },
        )
        .size_full();

        div().size_full().overflow_hidden().child(canvas)
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,
}

fn main() {
    let args = Args::parse();

    Application::new().run(move |cx: &mut App| {
        cx.activate(true);
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| {
                Program(Arc::new(RenderTree::from_scan_tree(
                    scan_dir(&args.entrypoint),
                    args.entrypoint.into(),
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                    true,
                )))
            })
        })
        .unwrap();
    });
}
