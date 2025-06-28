use clap::Parser;
use fs_tree::{scan_dir, Tree};
use gpui::{div, hash, prelude::*, rgb, App, Application, DefiniteLength, Window, WindowOptions};
use std::path::{Path, PathBuf};

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

fn slight_color_change(base: u32, random: u16) -> u32 {
    let dr = (((random >> 8) & 0xf) as i8) - 8;
    let dg = (((random >> 4) & 0xf) as i8) - 8;
    let db = ((random & 0xf) as i8) - 8;

    let pr = ((base >> 16) & 0xff) as i16;
    let pg = ((base >> 8) & 0xff) as i16;
    let pb = (base & 0xff) as i16;

    let r = (pr + dr as i16).clamp(0, 255) as u32;
    let g = (pg + dg as i16).clamp(0, 255) as u32;
    let b = (pb + db as i16).clamp(0, 255) as u32;

    (r << 16) | (g << 8) | b
}

fn render_children(tree: &Tree, color: u32, rotation: Rotation) -> Vec<impl IntoElement> {
    match tree {
        Tree::Leaf(_) => vec![div().size_full().bg(rgb(color))],
        Tree::Node { children, size } => children
            .into_iter()
            .map(|(name, subtree)| {
                let color = slight_color_change(color, hash(&name) as u16);
                let size = DefiniteLength::Fraction(subtree.get_size() as f32 / *size as f32);
                let element = match rotation {
                    Rotation::Vertical => {
                        div().overflow_hidden().flex().flex_row().w_full().h(size)
                    }
                    Rotation::Horizontal => {
                        div().overflow_hidden().flex().flex_col().h_full().w(size)
                    }
                };
                element.children(render_children(subtree, color, rotation.rotate()))
            })
            .collect(),
    }
}

struct FsTree {
    tree: Tree,
}

impl FsTree {
    pub fn new(entrypoint: &Path) -> Self {
        Self {
            tree: scan_dir(entrypoint),
        }
    }
}

impl Render for FsTree {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_row()
            .children(render_children(&self.tree, 0x7f7f7f, Rotation::Horizontal))
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
            cx.new(|_| FsTree::new(&args.entrypoint))
        })
        .unwrap();
    });
}
