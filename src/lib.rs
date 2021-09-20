pub mod example_opts;

use clap::App;
use iced::{
    button, container, text_input, tooltip::Position, Align, Background, Button, Color, Column,
    Command, Length, Row, Space, Text, TextInput, Tooltip,
};
use std::collections::HashMap;

pub struct Klask<'a> {
    app: App<'a>,
    clap_widget: AppWidget,
    run_state: button::State,
}

struct AppWidget {
    args: Vec<Arg>,
    subcommands: HashMap<String, AppWidget>,
}

impl AppWidget {
    fn new(app: &App) -> Self {
        Self {
            args: app
                .get_arguments()
                .filter(|a| a.get_name() != "help")
                .filter(|a| a.get_name() != "version")
                .map(|a| Arg {
                    name: a.get_name().into(),
                    desc: a.get_about().unwrap_or(&"").to_string(),
                    kind: if a.is_set(clap::ArgSettings::MultipleOccurrences) {
                        ArgKind::Occurences {
                            value: 0,
                            increment_state: Default::default(),
                            decrement_state: Default::default(),
                        }
                    } else {
                        ArgKind::Text {
                            value: "".into(),
                            default: a
                                .get_default_values()
                                .first()
                                .map_or_else(Default::default, |s| {
                                    s.to_string_lossy().into_owned()
                                }),
                            state: Default::default(),
                        }
                    },
                })
                .collect(),
            subcommands: HashMap::new(),
        }
    }

    fn view(&mut self) -> Column<'_, Message> {
        self.args
            .iter_mut()
            .fold(Column::new().padding(10).spacing(10), |col, arg| {
                col.push(arg.view())
            })
    }
}

struct Arg {
    name: String,
    desc: String,
    kind: ArgKind,
}

impl Arg {
    fn view(&mut self) -> Row<'_, Message> {
        let Arg { name, desc, kind } = self;

        match kind {
            ArgKind::Text {
                value,
                default,
                state,
            } => Row::new()
                .align_items(Align::Center)
                .push(
                    Tooltip::new(Text::new(name.clone()), desc, Position::FollowCursor)
                        .style(MyStyle),
                )
                .push(Space::with_width(Length::Units(10)))
                .push(
                    TextInput::new(state, default, &value, {
                        let name = name.clone();
                        move |value| Message::UpdateText {
                            name: name.clone(),
                            value,
                        }
                    })
                    .padding(4),
                ),
            ArgKind::Occurences {
                value,
                increment_state,
                decrement_state,
            } => Row::new()
                .align_items(Align::Center)
                .push(
                    Tooltip::new(Text::new(name.clone()), desc, Position::FollowCursor)
                        .style(MyStyle),
                )
                .push(Space::with_width(Length::Units(10)))
                .push(
                    Button::new(
                        decrement_state,
                        Text::new("-").horizontal_alignment(iced::HorizontalAlignment::Center),
                    )
                    .width(Length::Units(20))
                    .on_press(Message::UpdateOccurences {
                        name: name.clone(),
                        value: -1,
                    }),
                )
                .push(Space::with_width(Length::Units(5)))
                .push(Text::new(value.to_string()))
                .push(Space::with_width(Length::Units(5)))
                .push(
                    Button::new(
                        increment_state,
                        Text::new("+").horizontal_alignment(iced::HorizontalAlignment::Center),
                    )
                    .width(Length::Units(20))
                    .on_press(Message::UpdateOccurences {
                        name: name.clone(),
                        value: 1,
                    }),
                ),
        }
    }
}

enum ArgKind {
    Text {
        value: String,
        default: String,
        state: text_input::State,
    },
    Occurences {
        value: i32,
        increment_state: button::State,
        decrement_state: button::State,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateText { name: String, value: String },
    UpdateOccurences { name: String, value: i32 },
    Run,
    FinisedRun,
}

struct MyStyle;
impl container::StyleSheet for MyStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.8, 0.8))),
            border_width: 2.0,
            border_radius: 5.0,
            border_color: Color::BLACK,
            ..Default::default()
        }
    }
}

impl<'a> iced::Application for Klask<'a> {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = App<'a>;

    fn new(app: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                clap_widget: AppWidget::new(&app),
                run_state: Default::default(),
                app,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        self.app.get_name().into()
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut iced::Clipboard,
    ) -> iced::Command<Self::Message> {
        // match message {
        //     Message::UpdateText { name, value } => {
        //         for Arg { name: n, kind, .. } in self.args.iter_mut() {
        //             if name == *n {
        //                 if let ArgKind::Text { value: v, .. } = kind {
        //                     *v = value;
        //                     return Command::none();
        //                 }
        //             }
        //         }
        //     }
        //     Message::UpdateOccurences { name, value } => {
        //         for Arg { name: n, kind, .. } in self.args.iter_mut() {
        //             if name == *n {
        //                 if let ArgKind::Occurences { value: v, .. } = kind {
        //                     *v += value;
        //                     if *v < 0 {
        //                         *v = 0
        //                     }
        //                     return Command::none();
        //                 }
        //             }
        //         }
        //     }
        //     Message::Run => {
        //         return async { Message::FinisedRun }.into();
        //     }
        //     Message::FinisedRun => {
        //         println!("Runned {:?}", self.app.get_bin_name());
        //         return Command::none();
        //     }
        // };
        panic!()
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        Column::new()
            .push(self.clap_widget.view())
            .push(Button::new(&mut self.run_state, Text::new("Run!")).on_press(Message::Run))
            .into()
    }
}
