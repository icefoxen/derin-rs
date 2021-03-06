extern crate derin;
extern crate png;

use derin::{LoopFlow, Window, WindowAttributes};
use derin::layout::{Align, Align2, Margins, LayoutHorizontal};
use derin::container::SingleContainer;
use derin::widgets::{Contents, Group, Label};
use derin::theme::{ThemeWidget, Image, RescaleRules};
use derin::theme::color::{Rgba, Nu8};
use derin::geometry::DimsBox;

use std::rc::Rc;

fn main() {
    let group = Group::new(
        SingleContainer::new(Label::new(Contents::Image("AddIcon".to_string()))),
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let mut theme = derin::theme::Theme::default();
    theme.insert_widget(
        "AddIcon".to_string(),
        ThemeWidget {
            text: None,
            image: Some(Rc::new(Image {
                pixels: {
                    let image_png = png::Decoder::new(::std::io::Cursor::new(&include_bytes!("plus_icon.png")[..]));
                    let (info, mut reader) = image_png.read_info().unwrap();
                    // Allocate the output buffer.
                    let mut image = vec![0; info.buffer_size()];
                    reader.next_frame(&mut image).unwrap();
                    Rgba::slice_from_raw(Nu8::slice_from_raw(&image)).to_vec()
                },
                dims: DimsBox::new2(32, 32),
                rescale: RescaleRules::Align(Align2::new(Align::Center, Align::Center))
            }))
        }
    );

    let window_attributes = WindowAttributes {
        dimensions: Some((64, 64)),
        title: "Custom Icon".to_string(),
        ..WindowAttributes::default()
    };

    let mut window = unsafe{ Window::new(window_attributes, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |_: (), _, _| {
            LoopFlow::Continue
        },
        |_, _| None
    );
}
