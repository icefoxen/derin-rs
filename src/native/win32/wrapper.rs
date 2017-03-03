use ui::{Control, Parent, Node, ChildId, NodeProcessor, NodeProcessorAT, NodeDataWrapper};
use ui::intrinsics::TextButton;
use dww::*;
use dle::{Widget, Container, LayoutEngine, WidgetData, WidgetConstraintSolver, SolveError};
use dle::hints::WidgetHints;
use dct::events::{MouseEvent, MouseButton};
use dct::geometry::OffsetRect;

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

use super::NodeTraverser;

pub struct TextButtonNodeData<I>( UnsafeSubclassWrapper<PushButtonBase, TextButtonSubclass<I>> )
        where I: AsRef<str> + Control;

pub struct WidgetGroupNodeData<I>( UnsafeSubclassWrapper<BlankBase, WidgetGroupSubclass<I>> )
        where I: Parent<()>;

pub struct TextLabelNodeData<S>( UnsafeSubclassWrapper<TextLabelBase, TextLabelSubclass<S>> )
        where S: AsRef<str>;

impl<I: AsRef<str> + Control> WidgetDataContainer for TextButtonNodeData<I> {
    #[inline]
    fn get_widget_data(&self) -> WidgetData {
        self.0.subclass_data.mutable_data.borrow().widget_data
    }
}

impl<I: Parent<()>> WidgetDataContainer for WidgetGroupNodeData<I> {
    #[inline]
    fn get_widget_data(&self) -> WidgetData {
        self.0.subclass_data.widget_data
    }
}

impl<S: AsRef<str>> WidgetDataContainer for TextLabelNodeData<S> {
    #[inline]
    fn get_widget_data(&self) -> WidgetData {
        self.0.subclass_data.widget_data.get()
    }
}

impl<I: AsRef<str> + Control> NodeDataWrapper<I> for TextButtonNodeData<I> {
    fn from_node_data(node_data: I) -> TextButtonNodeData<I> {
        let button_window = WindowBuilder::default().build_push_button();
        let subclass = TextButtonSubclass::new(node_data);

        let wrapper = unsafe{ UnsafeSubclassWrapper::new(button_window, subclass) };
        TextButtonNodeData(wrapper)
    }

    fn inner(&self) -> &I {&self.0.subclass_data.node_data}
    fn inner_mut(&mut self) -> &mut I {&mut self.0.subclass_data.node_data}
    fn unwrap(self) -> I {self.0.subclass_data.node_data}
}

impl<I: Parent<()>> NodeDataWrapper<I> for WidgetGroupNodeData<I> {
    fn from_node_data(node_data: I) -> WidgetGroupNodeData<I> {
        let blank_window = WindowBuilder::default().build_blank();
        let subclass = WidgetGroupSubclass::new(node_data);

        let wrapper = unsafe{ UnsafeSubclassWrapper::new(blank_window, subclass) };
        WidgetGroupNodeData(wrapper)
    }

    fn inner(&self) -> &I {&self.0.subclass_data.node_data}
    fn inner_mut(&mut self) -> &mut I {&mut self.0.subclass_data.node_data}
    fn unwrap(self) -> I {self.0.subclass_data.node_data}
}

impl<S: AsRef<str>> NodeDataWrapper<S> for TextLabelNodeData<S> {
    fn from_node_data(text: S) -> TextLabelNodeData<S> {
        let label_window = WindowBuilder::default().build_text_label();
        let subclass = TextLabelSubclass::new(text);

        let wrapper = unsafe{ UnsafeSubclassWrapper::new(label_window, subclass) };
        TextLabelNodeData(wrapper)
    }

    fn inner(&self) -> &S {&self.0.subclass_data.text}
    fn inner_mut(&mut self) -> &mut S {&mut self.0.subclass_data.text}
    fn unwrap(self) -> S {self.0.subclass_data.text}
}



enum ButtonState {
    Released,
    Pressed,
    DoublePressed
}

impl Default for ButtonState {
    #[inline]
    fn default() -> ButtonState {
        ButtonState::Released
    }
}

struct TextButtonSubclass<I: AsRef<str> + Control> {
    node_data: I,
    mutable_data: RefCell<TBSMut>
}

#[derive(Default)]
struct TBSMut {
    widget_data: WidgetData,
    button_state: ButtonState
}

impl<I: AsRef<str> + Control> TextButtonSubclass<I> {
    #[inline]
    fn new(node_data: I) -> TextButtonSubclass<I> {
        TextButtonSubclass {
            node_data: node_data,
            mutable_data: RefCell::new(TBSMut::default())
        }
    }
}

impl<B, I> Subclass<B> for TextButtonSubclass<I>
        where B: ButtonWindow,
              I: AsRef<str> + Control
{
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<B>, msg: Msg<()>) -> i64 {
        let ret = window.default_window_proc();
        let mut mutable_data = self.mutable_data.borrow_mut();

        match msg {
            Msg::Wm(wm) => match wm {
                Wm::MouseDown(_, _) => mutable_data.button_state = ButtonState::Pressed,
                Wm::MouseDoubleDown(_, _) => mutable_data.button_state = ButtonState::DoublePressed,
                Wm::MouseUp(button, point) => {
                    let action = match mutable_data.button_state {
                        ButtonState::Pressed       => self.node_data.on_mouse_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => self.node_data.on_mouse_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };

                    mutable_data.button_state = ButtonState::Released;
                },
                Wm::SetText(_) => mutable_data.widget_data.abs_size_bounds.min = window.get_ideal_size(),
                _ => ()
            },
            _ => ()
        }
        ret
    }
}


struct WidgetGroupSubclass<I: Parent<()>> {
    node_data: I,
    widget_data: WidgetData,
    layout_engine: LayoutEngine
}

impl<I: Parent<()>> WidgetGroupSubclass<I> {
    #[inline]
    fn new(node_data: I) -> WidgetGroupSubclass<I> {
        WidgetGroupSubclass {
            node_data: node_data,
            widget_data: WidgetData::default(),
            layout_engine: LayoutEngine::new()
        }
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow,
              I: Parent<()>
{
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<P>, msg: Msg<()>) -> i64 {
        if let Msg::Wm(Wm::GetSizeBounds(size_bounds)) = msg {
            *size_bounds = self.layout_engine.actual_size_bounds();
            0
        } else {
            window.default_window_proc()
        }
    }
}


struct TextLabelSubclass<S: AsRef<str>> {
    text: S,
    widget_data: Cell<WidgetData>
}

impl<S: AsRef<str>> TextLabelSubclass<S> {
    #[inline]
    fn new(text: S) -> TextLabelSubclass<S> {
        TextLabelSubclass {
            text: text,
            widget_data: Cell::default()
        }
    }
}

impl<W, S> Subclass<W> for TextLabelSubclass<S>
        where W: TextLabelWindow,
              S: AsRef<str>
{
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<W>, msg: Msg<()>) -> i64 {
        if let Msg::Wm(Wm::SetText(new_text)) = msg {
            let mut widget_data = self.widget_data.get();
            widget_data.abs_size_bounds.min = unsafe{ window.min_unclipped_rect_raw(new_text) };
            self.widget_data.set(widget_data);
        }
        window.default_window_proc()
    }
}



/// Newtype wrapper around parents to allow them to implement `Container` trait
struct ParentContainer<I>(I);

impl<I, NP> Parent<NP> for ParentContainer<I>
        where I: Parent<NP>,
              NP: NodeProcessorAT
{
    type ChildAction = I::ChildAction;
    type ChildLayout = I::ChildLayout;

    fn children(&mut self, np: NP) -> Result<(), NP::Error> {
        self.0.children(np)
    }
    fn child_layout(&self) -> I::ChildLayout {
        self.0.child_layout()
    }
}

impl<I> Container for ParentContainer<I>
        where for<'a> I: Parent<ConstraintSolverTraverser<'a>>
{
    fn update_widget_rects(&mut self, solver: WidgetConstraintSolver) {
        let traverser = ConstraintSolverTraverser {
            solver: solver
        };
        self.children(traverser).ok();
    }
}

struct ConstraintSolverTraverser<'a> {
    solver: WidgetConstraintSolver<'a>
}

impl<'s, W, N> NodeProcessor<W, N> for ConstraintSolverTraverser<'s>
        where W: NodeDataWrapper<N::Inner> + WidgetDataContainer + Window,
              N: Node<W>
{
    fn add_child<'a>(&'a mut self, _: ChildId, node: &'a mut N) -> Result<(), ()> {
        match self.solver.solve_widget_constraints(node.data().get_widget_data()) {
            Ok(rect) => {node.data().set_rect(rect); Ok(())},
            Err(SolveError::Abort) => Err(()),
            Err(SolveError::WidgetUnsolvable) => Ok(())
        }
    }
}

impl<'a> NodeProcessorAT for ConstraintSolverTraverser<'a> {
    type Error = ();
}

trait WidgetDataContainer {
    fn get_widget_data(&self) -> WidgetData;
}
