use ratatui::{DefaultTerminal, Frame, layout::{Constraint, Layout, Rect}, style::Style, text::{Line, Span}, widgets::{Block, BorderType}};
use ratatui::prelude::Stylize;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1);
    let horizontal = Layout::horizontal([Constraint::Percentage(20); 3]).spacing(1);
    let [top, main] = frame.area().layout(&vertical);
    let [left, middle, right] = main.layout(&horizontal);
        
    let title = Line::from_iter([
        Span::from("Block Widget").bold(),
        Span::from(" (Press 'q' to quit)"),
    ]);

    frame.render_widget(title.centered(), top);

    render_bordered_block(frame, left);
    render_styled_block(frame, middle);
    render_custom_bordered_block(frame, right);
}

/// Render a block with borders.
pub fn render_bordered_block(frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title("Bordered block");
    frame.render_widget(block, area);
}

/// Render a styled block.
pub fn render_styled_block(frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .style(Style::new().blue().on_black().bold().italic())
        .title("Styled block");
    frame.render_widget(block, area);
}

/// Render a block with custom borders.
pub fn render_custom_bordered_block(frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().red())
        .title("Custom borders");
    frame.render_widget(block, area);
}
