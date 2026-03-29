use std::rc::Rc;
use std::sync::Arc;

use call::{ActiveCall, Room};
use client::{User, proto::PeerId};
use gpui::{
    Hsla, IntoElement, MouseButton, Path, ScreenCaptureSource,
    canvas, point,
};
use gpui::{App, Task, Window};
use rpc::proto::{self};
use theme::ActiveTheme;
use ui::{
    Avatar, AvatarAudioStatusIndicator,
    Facepile, Tooltip, prelude::*,
};
use workspace::{ParticipantLocation, notifications::DetachAndPromptErr};

use crate::TitleBar;


pub fn toggle_screen_sharing(
    screen: anyhow::Result<Option<Rc<dyn ScreenCaptureSource>>>,
    window: &mut Window,
    cx: &mut App,
) {
    let call = ActiveCall::global(cx).read(cx);
    let toggle_screen_sharing = match screen {
        Ok(screen) => {
            let Some(room) = call.room().cloned() else {
                return;
            };

            room.update(cx, |room, cx| {
                let clicked_on_currently_shared_screen =
                    room.shared_screen_id().is_some_and(|screen_id| {
                        Some(screen_id)
                            == screen
                                .as_deref()
                                .and_then(|s| s.metadata().ok().map(|meta| meta.id))
                    });
                let should_unshare_current_screen = room.is_sharing_screen();
                let unshared_current_screen = should_unshare_current_screen.then(|| {
                    telemetry::event!(
                        "Screen Share Disabled",
                        room_id = room.id(),
                        channel_id = room.channel_id(),
                    );
                    room.unshare_screen(clicked_on_currently_shared_screen || screen.is_none(), cx)
                });
                if let Some(screen) = screen {
                    if !should_unshare_current_screen {
                        telemetry::event!(
                            "Screen Share Enabled",
                            room_id = room.id(),
                            channel_id = room.channel_id(),
                        );
                    }
                    cx.spawn(async move |room, cx| {
                        unshared_current_screen.transpose()?;
                        if !clicked_on_currently_shared_screen {
                            room.update(cx, |room, cx| room.share_screen(screen, cx))?
                                .await
                        } else {
                            Ok(())
                        }
                    })
                } else {
                    Task::ready(Ok(()))
                }
            })
        }
        Err(e) => Task::ready(Err(e)),
    };
    toggle_screen_sharing.detach_and_prompt_err("Sharing Screen Failed", window, cx, |e, _, _| Some(format!("{:?}\n\nPlease check that you have given Zed permissions to record your screen in Settings.", e)));
}

pub fn toggle_mute(cx: &mut App) {
    let call = ActiveCall::global(cx).read(cx);
    if let Some(room) = call.room().cloned() {
        room.update(cx, |room, cx| {
            let operation = if room.is_muted() {
                "Microphone Enabled"
            } else {
                "Microphone Disabled"
            };
            telemetry::event!(
                operation,
                room_id = room.id(),
                channel_id = room.channel_id(),
            );

            room.toggle_mute(cx)
        });
    }
}

pub fn toggle_deafen(cx: &mut App) {
    if let Some(room) = ActiveCall::global(cx).read(cx).room().cloned() {
        room.update(cx, |room, cx| room.toggle_deafen(cx));
    }
}

fn render_color_ribbon(color: Hsla) -> impl Element {
    canvas(
        move |_, _, _| {},
        move |bounds, _, window, _| {
            let height = bounds.size.height;
            let horizontal_offset = height;
            let vertical_offset = height / 2.0;
            let mut path = Path::new(bounds.bottom_left());
            path.curve_to(
                bounds.origin + point(horizontal_offset, vertical_offset),
                bounds.origin + point(px(0.0), vertical_offset),
            );
            path.line_to(bounds.top_right() + point(-horizontal_offset, vertical_offset));
            path.curve_to(
                bounds.bottom_right(),
                bounds.top_right() + point(px(0.0), vertical_offset),
            );
            path.line_to(bounds.bottom_left());
            window.paint_path(path, color);
        },
    )
    .h_1()
    .w_full()
}

impl TitleBar {
    pub(crate) fn render_collaborator_list(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let room = ActiveCall::global(cx).read(cx).room().cloned();
        let current_user = self.user_store.read(cx).current_user();
        let client = self.client.clone();
        let project_id = self.project.read(cx).remote_id();
        let workspace = self.workspace.upgrade();

        h_flex()
            .id("collaborator-list")
            .w_full()
            .gap_1()
            .overflow_x_scroll()
            .when_some(
                current_user.zip(client.peer_id()).zip(room),
                |this, ((current_user, peer_id), room)| {
                    let player_colors = cx.theme().players();
                    let room = room.read(cx);
                    let mut remote_participants =
                        room.remote_participants().values().collect::<Vec<_>>();
                    remote_participants.sort_by_key(|p| p.participant_index.0);

                    let current_user_face_pile = self.render_collaborator(
                        &current_user,
                        peer_id,
                        true,
                        room.is_speaking(),
                        room.is_muted(),
                        None,
                        room,
                        project_id,
                        &current_user,
                        cx,
                    );

                    this.children(current_user_face_pile.map(|face_pile| {
                        v_flex()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .child(face_pile)
                            .child(render_color_ribbon(player_colors.local().cursor))
                    }))
                    .children(remote_participants.iter().filter_map(|collaborator| {
                        let player_color =
                            player_colors.color_for_participant(collaborator.participant_index.0);
                        let is_following = workspace
                            .as_ref()?
                            .read(cx)
                            .is_being_followed(collaborator.peer_id);
                        let is_present = project_id.is_some_and(|project_id| {
                            collaborator.location
                                == ParticipantLocation::SharedProject { project_id }
                        });

                        let facepile = self.render_collaborator(
                            &collaborator.user,
                            collaborator.peer_id,
                            is_present,
                            collaborator.speaking,
                            collaborator.muted,
                            is_following.then_some(player_color.selection),
                            room,
                            project_id,
                            &current_user,
                            cx,
                        )?;

                        Some(
                            v_flex()
                                .id(("collaborator", collaborator.user.id))
                                .child(facepile)
                                .child(render_color_ribbon(player_color.cursor))
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, |_, window, _| {
                                    window.prevent_default()
                                })
                                .on_click({
                                    let peer_id = collaborator.peer_id;
                                    cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();

                                        this.workspace
                                            .update(cx, |workspace, cx| {
                                                if is_following {
                                                    workspace.unfollow(peer_id, window, cx);
                                                } else {
                                                    workspace.follow(peer_id, window, cx);
                                                }
                                            })
                                            .ok();
                                    })
                                })
                                .occlude()
                                .tooltip({
                                    let login = collaborator.user.github_login.clone();
                                    Tooltip::text(format!("Follow {login}"))
                                }),
                        )
                    }))
                },
            )
    }

    fn render_collaborator(
        &self,
        user: &Arc<User>,
        peer_id: PeerId,
        is_present: bool,
        is_speaking: bool,
        is_muted: bool,
        leader_selection_color: Option<Hsla>,
        room: &Room,
        project_id: Option<u64>,
        current_user: &Arc<User>,
        cx: &App,
    ) -> Option<Div> {
        if room.role_for_user(user.id) == Some(proto::ChannelRole::Guest) {
            return None;
        }

        const FACEPILE_LIMIT: usize = 3;
        let followers = project_id.map_or(&[] as &[_], |id| room.followers_for(peer_id, id));
        let extra_count = followers.len().saturating_sub(FACEPILE_LIMIT);

        Some(
            div()
                .m_0p5()
                .p_0p5()
                // When the collaborator is not followed, still draw this wrapper div, but leave
                // it transparent, so that it does not shift the layout when following.
                .when_some(leader_selection_color, |div, color| {
                    div.rounded_sm().bg(color)
                })
                .child(
                    Facepile::empty()
                        .child(
                            Avatar::new(user.avatar_uri.clone())
                                .grayscale(!is_present)
                                .border_color(if is_speaking {
                                    cx.theme().status().info
                                } else {
                                    // We draw the border in a transparent color rather to avoid
                                    // the layout shift that would come with adding/removing the border.
                                    gpui::transparent_black()
                                })
                                .when(is_muted, |avatar| {
                                    avatar.indicator(
                                        AvatarAudioStatusIndicator::new(ui::AudioStatus::Muted)
                                            .tooltip({
                                                let github_login = user.github_login.clone();
                                                Tooltip::text(format!("{} is muted", github_login))
                                            }),
                                    )
                                }),
                        )
                        .children(followers.iter().take(FACEPILE_LIMIT).filter_map(
                            |follower_peer_id| {
                                let follower = room
                                    .remote_participants()
                                    .values()
                                    .find_map(|p| {
                                        (p.peer_id == *follower_peer_id).then_some(&p.user)
                                    })
                                    .or_else(|| {
                                        (self.client.peer_id() == Some(*follower_peer_id))
                                            .then_some(current_user)
                                    })?
                                    .clone();

                                Some(div().mt(-px(4.)).child(
                                    Avatar::new(follower.avatar_uri.clone()).size(rems(0.75)),
                                ))
                            },
                        ))
                        .children(if extra_count > 0 {
                            Some(
                                Label::new(format!("+{extra_count}"))
                                    .ml_1()
                                    .into_any_element(),
                            )
                        } else {
                            None
                        }),
                ),
        )
    }

}
