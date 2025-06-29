use clap::Parser;
use fs_tree::{render_tree::RenderTree, scan_tree::scan_dir};
use gpui::{div, prelude::*, rgb, App, Application, DefiniteLength, Window, WindowOptions};
use std::path::PathBuf;

enum Rotation {
    Vertical,
    Horizontal,
}

impl Rotation {
    fn rotate(&self) -> Self {
        match self {
            Rotation::Vertical => Rotation::Horizontal,
            Rotation::Horizontal => Rotation::Vertical,
        }
    }
}

fn render_children(tree: &RenderTree, rotation: Rotation) -> Vec<impl IntoElement> {
    match tree {
        RenderTree::File { color, .. } => vec![div().size_full().bg(rgb(*color))],
        RenderTree::Dir { children, size, .. } => children
            .into_iter()
            .map(|subtree| {
                let size = DefiniteLength::Fraction(subtree.get_size() as f32 / *size as f32);
                let element = match rotation {
                    Rotation::Vertical => {
                        div().overflow_hidden().flex().flex_row().w_full().h(size)
                    }
                    Rotation::Horizontal => {
                        div().overflow_hidden().flex().flex_col().h_full().w(size)
                    }
                };
                element.children(render_children(subtree, rotation.rotate()))
            })
            .collect(),
    }
}

struct Program(pub RenderTree);

impl Render for Program {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_row()
            .children(render_children(&self.0, Rotation::Horizontal))
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
                Program(RenderTree::from_scan_tree(
                    scan_dir(&args.entrypoint),
                    args.entrypoint.into(),
                ))
            })
        })
        .unwrap();
    });
}
