mod client;

use anyhow::Result;
use client::TelevideoClient;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::io;

struct App {
    client: TelevideoClient,
    current_page: u16,
    current_part: u16,
    page_input_buffer: String,
    image_state: Option<StatefulProtocol>,
    picker: Picker,
    error: Option<String>,
    message: Option<String>,
    loading: bool,
}

impl App {
    fn new() -> Self {
        // Try to query the terminal for font size, or use a reasonable default
        // If your terminal is iTerm2, this should work automatically
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
            // Fallback to a more typical font size
            // These values represent width x height in pixels per character cell
            Picker::from_fontsize((10, 20))
        });
        Self {
            client: TelevideoClient::new(),
            current_page: 100,
            current_part: 1,
            page_input_buffer: String::new(),
            image_state: None,
            picker,
            error: None,
            message: None,
            loading: false,
        }
    }

    fn load_page(&mut self, page: u16, part: u16) {
        self.loading = true;
        self.error = None;
        self.message = None;

        match self.client.fetch_page(page, part) {
            Ok(bytes) => {
                // Load image from bytes
                match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        self.current_page = page;
                        self.current_part = part;
                        // Create a resizable protocol that will adapt to the render area
                        let protocol = self.picker.new_resize_protocol(img);
                        self.image_state = Some(protocol);
                        self.loading = false;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load image: {}", e));
                        self.loading = false;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("{}", e));
                self.loading = false;
            }
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.load_page(100, 1);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(())
                    }
                    KeyCode::Char('c') => {
                        app.client.clear_cache();
                        app.message = Some("Cache cleared!".to_string());
                        app.load_page(app.current_page, app.current_part);
                    }
                    KeyCode::Left => {
                        if app.current_page > 100 {
                            app.load_page(app.current_page - 1, 1);
                        }
                    }
                    KeyCode::Right => {
                        if app.current_page < 899 {
                            app.load_page(app.current_page + 1, 1);
                        }
                    }
                    KeyCode::Up => {
                        if app.current_part > 1 {
                            app.load_page(app.current_page, app.current_part - 1);
                        }
                    }
                    KeyCode::Down => {
                        app.load_page(app.current_page, app.current_part + 1);
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        app.page_input_buffer.push(c);
                    }
                    KeyCode::Enter => {
                        if !app.page_input_buffer.is_empty() {
                            if let Ok(page) = app.page_input_buffer.parse::<u16>() {
                                if (100..=899).contains(&page) {
                                    app.load_page(page, 1);
                                } else {
                                    app.message = Some("Page must be between 100-899".to_string());
                                }
                            }
                            app.page_input_buffer.clear();
                        }
                    }
                    KeyCode::Backspace => {
                        app.page_input_buffer.pop();
                    }
                    KeyCode::Esc => {
                        app.page_input_buffer.clear();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    let header_text = format!("  TELEVIDEO RAI - Page {}{}",
        app.current_page,
        if app.current_part > 1 { format!(".{}", app.current_part) } else { String::new() }
    );

    let header_right = if let Some(ref err) = app.error {
        format!("ERROR: {}  ", err)
    } else if app.loading {
        "Loading...  ".to_string()
    } else {
        String::new()
    };

    let header_line = create_bar(&header_text, &header_right, size.width);
    let header = Paragraph::new(header_line)
        .style(Style::default().bg(Color::Rgb(0, 0, 128)).fg(Color::White));
    f.render_widget(header, chunks[0]);

    let content_area = chunks[1];
    if !app.loading {
        if let Some(ref mut image_state) = app.image_state {
            // Use Scale mode to resize the image to fill the available space
            // The widget will maintain aspect ratio and center it automatically
            let image_widget = StatefulImage::default()
                .resize(Resize::Scale(None));
            f.render_stateful_widget(image_widget, content_area, image_state);
        }
    } else {
        let loading = Paragraph::new("Loading page...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default());
        f.render_widget(loading, content_area);
    }

    let footer_text = if let Some(ref msg) = app.message {
        format!("  {}", msg)
    } else if !app.page_input_buffer.is_empty() {
        format!("  Go to page: {}_", app.page_input_buffer)
    } else {
        "  [← /→ ] Page  [↑/↓] Sub-page  [0-9] Jump to page  [q] Quit  [c] Clear cache".to_string()
    };

    let footer_line = create_bar(&footer_text, "", size.width);
    let footer = Paragraph::new(footer_line)
        .style(Style::default().bg(Color::Rgb(0, 0, 128)).fg(Color::White));
    f.render_widget(footer, chunks[2]);
}

fn create_bar(left: &str, right: &str, width: u16) -> Line<'static> {
    let total_len = left.len() + right.len();
    let padding = if total_len < width as usize {
        " ".repeat(width as usize - total_len)
    } else {
        String::new()
    };

    Line::from(vec![
        Span::styled(
            format!("{}{}{}", left, padding, right),
            Style::default().bg(Color::Rgb(0, 0, 128)).fg(Color::White),
        ),
    ])
}

