// SPDX-License-Identifier: GPL-3.0-or-later
//
// High-score table shown inside the scores dialog.

use crate::storage::{Score, Scores};
use crate::theme::{ThemePalette, tokens};
use crate::ui::status::format_time;
use crate::ui::style::{FONT_BODY, FONT_CAPTION};
use bevy::feathers::theme::{ThemeTextColor, ThemedText, ThemeToken};
use bevy::prelude::*;
use bevy::ui::{Overflow, OverflowAxis, ScrollPosition, rem};
use bevy::ui_widgets::ScrollArea;
use std::collections::BTreeMap;

pub fn contents(scores: &Scores, _palette: &ThemePalette) -> Box<dyn SceneList> {
    let mut groups: BTreeMap<(usize, usize, usize), Vec<&Score>> = BTreeMap::new();
    for score in &scores.0 {
        groups
            .entry((score.width, score.height, score.mines))
            .or_default()
            .push(score);
    }
    let mut rows: Vec<Box<dyn SceneList>> = Vec::new();
    if groups.is_empty() {
        rows.push(line(
            "暂无成绩。完成一局游戏即可记录首个成绩。",
            FONT_BODY,
            tokens::TEXT_SECONDARY,
        ));
    } else {
        for ((width, height, mines), list) in groups {
            rows.push(line(
                format!("{width} × {height} · {mines} 雷"),
                FONT_BODY,
                tokens::TEXT_MAIN,
            ));
            for (rank, score) in list.iter().take(10).enumerate() {
                rows.push(line(
                    format!("{:>2}. {}", rank + 1, format_time(score.seconds)),
                    FONT_CAPTION,
                    tokens::TEXT_SECONDARY,
                ));
            }
            rows.push(Box::new(bsn! { Node { height: rem(0.25) } }));
        }
    }
    Box::new(bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: rem(0.15),
            max_height: rem(15),
            padding: UiRect::right(rem(0.3)),
            overflow: Overflow {
                x: OverflowAxis::Clip,
                y: OverflowAxis::Scroll,
            },
        }
        ScrollPosition(Vec2::ZERO)
        ScrollArea
        Children [{rows}]
    })
}

fn line(content: impl Into<String>, size: f32, token: ThemeToken) -> Box<dyn SceneList> {
    let content = content.into();
    Box::new(bsn! {
        Text({content})
        TextFont { font_size: FontSize::Rem(size) }
        ThemeTextColor({token}) ThemedText
    })
}
