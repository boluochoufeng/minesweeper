// SPDX-License-Identifier: GPL-3.0-or-later
//
// High-score table shown inside the scores dialog. The three standard boards
// are laid out side by side so all of them fit without scrolling; custom board
// configurations wrap onto additional rows.

use crate::storage::{Score, Scores};
use crate::theme::{ThemePalette, tokens};
use crate::ui::status::format_time;
use crate::ui::style::{FONT_BODY, FONT_CAPTION};
use bevy::feathers::theme::{ThemeTextColor, ThemedText, ThemeToken};
use bevy::prelude::*;
use bevy::ui::{Overflow, OverflowAxis, ScrollPosition, rem};
use bevy::ui_widgets::ScrollArea;
use std::collections::BTreeMap;

/// Standard boards always get a column, even before anyone sets a record.
const STANDARD_BOARDS: [(usize, usize, usize); 3] = [(8, 8, 10), (16, 16, 40), (30, 16, 99)];

pub fn contents(scores: &Scores, _palette: &ThemePalette) -> Box<dyn SceneList> {
    let mut groups: BTreeMap<(usize, usize, usize), Vec<&Score>> = BTreeMap::new();
    for score in &scores.0 {
        groups
            .entry((score.width, score.height, score.mines))
            .or_default()
            .push(score);
    }
    let mut keys: Vec<(usize, usize, usize)> = STANDARD_BOARDS.to_vec();
    for key in groups.keys() {
        if !keys.contains(key) {
            keys.push(*key);
        }
    }
    let columns: Vec<Box<dyn SceneList>> = keys
        .into_iter()
        .map(|(width, height, mines)| board_column(&groups, width, height, mines))
        .collect();
    Box::new(bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            max_height: rem(15),
            padding: UiRect::right(rem(0.3)),
            overflow: Overflow {
                x: OverflowAxis::Clip,
                y: OverflowAxis::Scroll,
            },
        }
        ScrollPosition(Vec2::ZERO)
        ScrollArea
        Children [
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Start,
                column_gap: rem(0.7),
                row_gap: rem(1.0),
            }
            Children [{columns}]
        ]
    })
}

fn board_column(
    groups: &BTreeMap<(usize, usize, usize), Vec<&Score>>,
    width: usize,
    height: usize,
    mines: usize,
) -> Box<dyn SceneList> {
    let mut rows: Vec<Box<dyn SceneList>> = vec![
        line(format!("{width} × {height}"), FONT_BODY, tokens::TEXT_MAIN),
        line(format!("{mines} 雷"), FONT_CAPTION, tokens::TEXT_SECONDARY),
    ];
    match groups.get(&(width, height, mines)) {
        Some(list) if !list.is_empty() => {
            for (rank, score) in list.iter().take(10).enumerate() {
                let token = if rank == 0 {
                    tokens::TEXT_ACCENT
                } else {
                    tokens::TEXT_SECONDARY
                };
                rows.push(line(
                    format!("{:>2}. {}", rank + 1, format_time(score.seconds)),
                    FONT_CAPTION,
                    token,
                ));
            }
        }
        _ => rows.push(line("暂无成绩", FONT_CAPTION, tokens::TEXT_SECONDARY)),
    }
    Box::new(bsn! {
        Node {
            width: rem(7.6),
            flex_direction: FlexDirection::Column,
            row_gap: rem(0.12),
        }
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
