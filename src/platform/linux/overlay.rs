use crate::core::element::{HintItem, UIElement};
use crate::core::hint::{HintMatcher, MatchResult};
use crate::platform::OverlayBackend;
use anyhow::Result;
use std::io::{self, Write};

pub struct LinuxOverlay;

impl LinuxOverlay {
    pub fn new() -> Self {
        Self
    }
}

impl OverlayBackend for LinuxOverlay {
    fn show_overlay(&mut self, hints: &[HintItem]) -> Result<Option<UIElement>> {
        if hints.is_empty() {
            println!("No interactive elements to display.");
            return Ok(None);
        }

        println!("\n=== VIMIUM DESKTOP HINTS (Active Elements) ===");
        for item in hints {
            println!(
                "  [{:^4}] {:<12} '{}' at ({}, {}) [{}x{}]",
                item.hint.to_uppercase(),
                item.element.role,
                item.element.name,
                item.element.rect.x,
                item.element.rect.y,
                item.element.rect.width,
                item.element.rect.height,
            );
        }
        println!("==============================================\n");

        let mut matcher = HintMatcher::new(hints.to_vec());

        print!("Type hint letters (or 'q' to cancel): ");
        io::stdout().flush()?;

        let stdin = io::stdin();
        let mut line = String::new();
        while stdin.read_line(&mut line)? > 0 {
            let input = line.trim();
            if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("esc") {
                println!("Canceled.");
                return Ok(None);
            }

            for c in input.chars() {
                match matcher.feed_char(c) {
                    MatchResult::Selected(element) => {
                        println!("\n[Selected] '{}' ({})", element.name, element.role);
                        return Ok(Some(element));
                    }
                    MatchResult::Narrowed(count) => {
                        println!("Narrowed down to {} hint(s)...", count);
                    }
                    MatchResult::Mismatch => {
                        println!("No match for '{}'", c);
                    }
                }
            }
            line.clear();
            print!("Continue typing hint: ");
            io::stdout().flush()?;
        }

        Ok(None)
    }
}
