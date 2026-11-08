use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;

use crate::ui;

#[derive(Default)]
pub struct GuardPrompt(Vec<u8>);

impl AsyncWrite for GuardPrompt {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.0.is_empty() {
            let text = String::from_utf8_lossy(&self.0).into_owned();
            self.0.clear();
            ui::ask(&question(&text));
        }

        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

fn question(text: &str) -> String {
    let (kind, detail) = text
        .split_once(':')
        .map_or((text.trim(), ""), |(kind, detail)| {
            (kind.trim(), detail.trim())
        });

    match kind {
        "email" if !detail.is_empty() => {
            format!("Steam Guard code sent to your email ({detail}): ")
        }
        "email" => "Steam Guard code sent to your email: ".into(),
        "device code" => {
            "Steam Guard code from the Steam Mobile app (or approve the sign-in in the app): "
                .into()
        }
        _ => "Steam Guard code: ".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::question;

    #[test]
    fn formats_prompts() {
        assert_eq!(
            question("email: riseup.net"),
            "Steam Guard code sent to your email (riseup.net): "
        );
        assert!(
            question("device code: ").starts_with("Steam Guard code from the Steam Mobile app")
        );
        assert_eq!(question("something"), "Steam Guard code: ");
    }
}
