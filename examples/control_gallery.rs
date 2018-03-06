extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate png;

use derin::{LoopFlow, Window, WindowAttributes};
use derin::layout::{Margins, LayoutHorizontal, LayoutVertical};
use derin::widgets::{Button, EditBox, Group, Label};
use derin::theme::{ThemeWidget, Image, RescaleRules};
use derin::theme::color::{Rgba, Nu8};
use derin::geometry::DimsBox;

use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryEvent {
    NewButton
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: Group<NestedContainer, LayoutVertical>
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    label: Label,
    edit_box: EditBox,
    #[derin(collection = "Button<Option<GalleryEvent>>")]
    buttons: Vec<Button<Option<GalleryEvent>>>
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new("Add Button".to_string(), Some(GalleryEvent::NewButton)),
            nested: Group::new(
                NestedContainer {
                    label: Label::new("Nested Container".to_string()),
                    edit_box: EditBox::new("A Text Box".to_string()),
                    buttons: Vec::new(),
                },
                LayoutVertical::new(Margins::new(8, 8, 8, 8), Default::default())
            )
        },
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
                rescale: RescaleRules::Stretch
            }))
        }
    );

    let window_attributes = WindowAttributes {
        dimensions: Some((512, 512)),
        title: "Derin Control Gallery".to_string(),
        ..WindowAttributes::default()
    };

    let mut window = unsafe{ Window::new(window_attributes, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |event, root, _| {
            root.container_mut().nested.container_mut().buttons.push(Button::new("An added button".to_string(), None));
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
