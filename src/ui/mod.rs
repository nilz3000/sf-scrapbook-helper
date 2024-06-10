use chrono::Local;
use iced::{
    alignment::Horizontal,
    theme,
    widget::{
        self, button, checkbox, column, container, horizontal_space, pick_list,
        row, text,
    },
    Alignment, Element, Length,
};
use iced_aw::number_input;

use self::{scrapbook::view_scrapbook, underworld::view_underworld};
use crate::{
    config::AvailableTheme, get_server_code, message::Message,
    player::AccountStatus, server::CrawlingStatus, top_bar, AccountIdent,
    AccountPage, Helper, View,
};

mod scrapbook;
pub mod underworld;

impl Helper {
    pub fn view_current_page(&self) -> Element<Message> {
        let view: Element<Message> = match self.current_view {
            View::Account { ident, page } => self.view_account(ident, page),
            View::Login => self
                .login_state
                .view(&self.config.accounts, self.has_accounts()),
            View::Overview => self.view_overview(),
            View::Settings => self.view_settings(),
        };
        let main_part = container(view).width(Length::Fill).center_x();
        let mut res = column!();

        if self.should_update {
            let dl_button =  button("Download").on_press(
                Message::OpenLink("https://github.com/the-marenga/sf-scrapbook-helper/releases/latest".to_string())
            );

            let ignore_button = button("Ignore")
                .on_press(Message::UpdateResult(false))
                .style(theme::Button::Destructive);

            let update_msg = row!(
                horizontal_space(),
                text("A new Version is available!").size(20),
                dl_button,
                horizontal_space(),
                ignore_button,
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .width(Length::Fill)
            .padding(15);

            res = res.push(update_msg);
        }
        res.push(main_part).into()
    }

    fn view_account(
        &self,
        ident: AccountIdent,
        page: AccountPage,
    ) -> Element<Message> {
        let Some((server, player)) = self.servers.get_ident(&ident) else {
            return self
                .login_state
                .view(&self.config.accounts, self.has_accounts());
        };

        let selection = |this_page: AccountPage| -> Element<Message> {
            button(text(format!("{this_page:?}")))
                .on_press(Message::ViewSubPage {
                    player: player.ident,
                    page: this_page,
                })
                .padding(4)
                .style(if this_page == page {
                    theme::Button::Primary
                } else {
                    theme::Button::Secondary
                })
                .into()
        };

        let top = row!(
            text(titlecase::titlecase(&player.name).to_string()).size(20),
            text(get_server_code(&server.ident.url))
                .horizontal_alignment(iced::alignment::Horizontal::Right)
                .size(20),
            selection(AccountPage::Scrapbook),
            selection(AccountPage::Underworld),
            button(text("Logout"))
                .on_press(Message::RemoveAccount {
                    ident: player.ident,
                })
                .padding(4)
                .style(theme::Button::Destructive)
        )
        .spacing(15)
        .align_items(Alignment::Center);

        let top_bar = top_bar(top.into(), Some(Message::ViewOverview));

        let middle = match page {
            AccountPage::Scrapbook => {
                view_scrapbook(server, player, &self.config, &self.class_images)
            }
            AccountPage::Underworld => view_underworld(
                server, player, self.config.max_threads, &self.config,
                &self.class_images,
            ),
        };

        let col_container = container(middle).center_y();

        column!(top_bar, col_container)
            .spacing(5)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }

    fn view_settings(&self) -> Element<Message> {
        let top_row = top_bar(
            text("Settings").size(20).into(),
            if self.has_accounts() {
                Some(Message::ViewOverview)
            } else {
                Some(Message::ViewLogin)
            },
        );
        use AvailableTheme::*;
        let all_themes = [
            Light, Dark, Dracula, Nord, SolarizedLight, SolarizedDark,
            GruvboxLight, GruvboxDark, CatppuccinLatte, CatppuccinFrappe,
            CatppuccinMacchiato, CatppuccinMocha, TokyoNight, TokyoNightStorm,
            TokyoNightLight, KanagawaWave, KanagawaDragon, KanagawaLotus,
            Moonfly, Nightfly, Oxocarbon,
        ];

        let theme_picker = pick_list(
            all_themes,
            Some(self.config.theme),
            Message::ChangeTheme,
        )
        .width(Length::Fixed(200.0));

        let theme_row =
            row!(text("Theme: ").width(Length::Fixed(100.0)), theme_picker)
                .width(Length::Fill)
                .align_items(Alignment::Center);

        let auto_fetch_hof = checkbox(
            "Fetch online HoF backup during login",
            self.config.auto_fetch_newest,
        )
        .on_toggle(Message::SetAutoFetch);

        let auto_poll =
            checkbox("Keep characters logged in", self.config.auto_poll)
                .on_toggle(Message::SetAutoPoll);

        let crawling_restrict = checkbox(
            "Show advanced crawling options",
            self.config.show_crawling_restrict,
        )
        .on_toggle(Message::AdvancedLevelRestrict);

        let show_class_icons =
            checkbox("Show class icons", self.config.show_class_icons)
                .on_toggle(Message::ShowClasses);

        let max_threads =
            number_input(self.config.max_threads, 50, Message::SetMaxThreads);

        let max_threads = row!("Max threads:", horizontal_space(), max_threads)
            .width(Length::Fill)
            .align_items(Alignment::Center);

        let settings_column = column!(
            theme_row, auto_fetch_hof, auto_poll, max_threads,
            crawling_restrict, show_class_icons
        )
        .width(Length::Fixed(300.0))
        .spacing(20);

        column!(top_row, settings_column)
            .spacing(20)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }

    fn view_overview(&self) -> Element<Message> {
        let top_bar = top_bar(
            text("Characters").size(20).into(),
            Some(Message::ViewLogin),
        );

        let mut accounts = column!()
            .padding(20)
            .spacing(5)
            .width(Length::Fill)
            .align_items(Alignment::Center);

        let mut servers: Vec<_> = self.servers.0.values().collect();
        servers.sort_by_key(|a| &a.ident.ident);
        for server in servers {
            let server_status: Box<str> = match &server.crawling {
                CrawlingStatus::Waiting => "Waiting".into(),
                CrawlingStatus::Restoring => "Restoring".into(),
                CrawlingStatus::CrawlingFailed(_) => "Error".into(),
                CrawlingStatus::Crawling {
                    que, player_info, ..
                } => {
                    let lock = que.lock().unwrap();
                    let remaining = lock.count_remaining();
                    let crawled = player_info.len();
                    let total = remaining + crawled;
                    drop(lock);
                    if crawled == total {
                        "Finished".into()
                    } else {
                        format!("{crawled}/{total}").into()
                    }
                }
            };

            let mut accs: Vec<_> = server.accounts.values().collect();
            accs.sort_by_key(|a| &a.name);
            for acc in accs {
                let status = acc.status.lock().unwrap();
                let mut info_row = row!().spacing(10.0);
                let status_width = 80.0;
                let status_text = |t: &str| text(t).width(status_width);

                let mut next_free_fight = None;

                match &*status {
                    AccountStatus::LoggingIn => {
                        info_row = info_row.push(status_text("Logging in"));
                    }
                    AccountStatus::Idle(_, gs) => {
                        next_free_fight = Some(gs.arena.next_free_fight);
                        info_row = info_row.push(status_text("Active"));
                    }
                    AccountStatus::Busy(gs, reason) => {
                        next_free_fight = Some(gs.arena.next_free_fight);
                        info_row = info_row.push(status_text(reason));
                    }
                    AccountStatus::FatalError(_) => {
                        info_row = info_row.push(status_text("Error!"));
                    }
                    AccountStatus::LoggingInAgain => {
                        info_row = info_row.push(status_text("Logging in"));
                    }
                };
                info_row = info_row
                    .push(text(get_server_code(&server.ident.url)).width(50.0));
                info_row = info_row.push(
                    text(titlecase::titlecase(acc.name.as_str()).to_string())
                        .width(200.0),
                );
                info_row = info_row.push(horizontal_space());

                let ff_width = 40.0;
                match next_free_fight {
                    None => {
                        let g = iced_aw::core::icons::bootstrap::icon_to_text(
                            iced_aw::Bootstrap::Question,
                        );
                        info_row = info_row.push(
                            g.width(ff_width)
                                .size(18.0)
                                .horizontal_alignment(Horizontal::Center),
                        );
                    }
                    Some(Some(x)) if x >= Local::now() => {
                        let secs = (x - Local::now()).num_seconds();
                        info_row = info_row.push(
                            text(format!("{secs}s"))
                                .width(ff_width)
                                .horizontal_alignment(Horizontal::Center),
                        );
                    }
                    Some(_) => {
                        let g = iced_aw::core::icons::bootstrap::icon_to_text(
                            iced_aw::Bootstrap::Check,
                        );
                        info_row = info_row.push(
                            g.width(ff_width)
                                .size(18.0)
                                .horizontal_alignment(Horizontal::Center),
                        );
                    }
                };

                let abs = if let Some(sbi) = &acc.scrapbook_info {
                    if sbi.auto_battle {
                        iced_aw::core::icons::bootstrap::icon_to_text(
                            iced_aw::Bootstrap::Check,
                        )
                    } else {
                        iced_aw::core::icons::bootstrap::icon_to_text(
                            iced_aw::Bootstrap::X,
                        )
                    }
                } else {
                    iced_aw::core::icons::bootstrap::icon_to_text(
                        iced_aw::Bootstrap::Question,
                    )
                };

                info_row = info_row.push(
                    abs.width(40.0)
                        .size(18.0)
                        .horizontal_alignment(Horizontal::Center),
                );
                info_row = info_row.push(text(&server_status).width(120.0));

                let b = button(info_row)
                    .on_press(Message::ShowPlayer { ident: acc.ident })
                    .width(Length::Fill)
                    .style(theme::Button::Secondary);
                accounts = accounts.push(b);
            }
        }

        if self.servers.len() > 0 {
            let add_button = button(
                text("+")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::ViewLogin)
            .style(theme::Button::Positive);
            accounts = accounts.push(add_button);
        }

        column!(top_bar, widget::scrollable(accounts))
            .spacing(50)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }
}
