use relm4::component;
use relm4::gtk;
use relm4::gtk::traits::BoxExt;
use relm4::gtk::traits::ButtonExt;
use relm4::gtk::traits::GtkWindowExt;
use relm4::gtk::traits::OrientableExt;
use relm4::ComponentParts;
use relm4::ComponentSender;
use relm4::RelmApp;
use relm4::SimpleComponent;
use relm4::WidgetPlus;

struct AppModel {
    counter: u8,
}

#[derive(Debug)]
enum AppInput {
    Decrement,
    Increment,
}

#[component]
impl SimpleComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type InitParams = u8;
    type Widgets = AppWidgets;

    fn init(
        params: Self::InitParams,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { counter: params };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _: ComponentSender<Self>) {
        use AppInput::*;

        match message {
            Decrement => self.counter = self.counter.saturating_sub(1),
            Increment => self.counter = self.counter.saturating_add(1),
        };
    }

    view! {
        gtk::Window {
            set_title: Some("Fimfarchive"),
            set_default_width: 320,
            set_default_height: 240,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,

                gtk::Button::with_label("Increment") {
                    connect_clicked[sender] => move |_| {
                        sender.input(AppInput::Increment)
                    },
                },

                gtk::Button::with_label("Decrement") {
                    connect_clicked[sender] => move |_| {
                        sender.input(AppInput::Decrement)
                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Count: {}", model.counter),
                    set_margin_all: 5,
                },
            }
        }
    }
}

fn main() {
    RelmApp::new("net.fimfarchive.reader").run::<AppModel>(0);
}
