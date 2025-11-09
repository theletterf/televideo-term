mod client;

use anyhow::Result;
use client::{TelevideoClient, TelevideoPage};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::io;

#[derive(PartialEq, Clone, Copy)]
enum DisplayMode {
    Text,
    Image,
}

struct App {
    client: TelevideoClient,
    current_page: u16,
    current_part: u16,
    page_input_buffer: String,
    content: Option<TelevideoPage>,
    image_state: Option<StatefulProtocol>,
    error: Option<String>,
    message: Option<String>,
    loading: bool,
    display_mode: DisplayMode,
    picker: Picker,
}

impl App {
    fn new_with_picker(picker: Picker) -> Self {
        Self {
            client: TelevideoClient::new(),
            current_page: 100,
            current_part: 1,
            page_input_buffer: String::new(),
            content: None,
            image_state: None,
            error: None,
            message: None,
            loading: false,
            display_mode: DisplayMode::Text,
            picker,
        }
    }

    fn load_page(&mut self, page: u16, part: u16) {
        self.loading = true;
        self.error = None;
        self.message = None;

        // Load text content
        let text_result = self.client.fetch_page(page, part);
        match text_result {
            Ok(page_content) => {
                self.content = Some(page_content);
            }
            Err(e) => {
                self.error = Some(format!("Text: {}", e));
            }
        }

        // Load image content
        let image_result = self.client.fetch_image(page, part);
        match image_result {
            Ok(img) => {
                // Convert DynamicImage to Protocol using the picker
                let protocol = self.picker.new_resize_protocol(img);
                self.image_state = Some(protocol);
            }
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(format!("Image: {}", e));
                }
            }
        }

        self.current_page = page;
        self.current_part = part;
        self.loading = false;
    }
}

fn main() -> Result<()> {
    // Create picker before entering raw mode to allow stdio queries
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
        Picker::from_fontsize((8, 16))
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_with_picker(picker);
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
                    KeyCode::Char('v') => {
                        app.display_mode = match app.display_mode {
                            DisplayMode::Text => DisplayMode::Image,
                            DisplayMode::Image => DisplayMode::Text,
                        };
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
        match app.display_mode {
            DisplayMode::Text => {
                if let Some(ref page_content) = app.content {
                    // Calculate vertical centering
                    let content_height = page_content.lines.len();
                    let available_height = content_area.height as usize;
                    let vertical_padding = if content_height < available_height {
                        (available_height - content_height) / 2
                    } else {
                        0
                    };

                    // Add vertical padding lines at the top
                    let mut all_lines: Vec<Line> = vec![Line::from(""); vertical_padding];

                    // Display the parsed content with horizontal centering
                    let content_lines: Vec<Line> = page_content
                        .lines
                        .iter()
                        .map(|s| {
                            // Center each line horizontally by adding padding
                            let terminal_width = content_area.width as usize;
                            let line_len = s.len();
                            if line_len < terminal_width {
                                let padding = (terminal_width - line_len) / 2;
                                let padded = format!("{}{}", " ".repeat(padding), s);
                                Line::from(padded)
                            } else {
                                Line::from(s.as_str())
                            }
                        })
                        .collect();

                    all_lines.extend(content_lines);

                    let text = Paragraph::new(all_lines)
                        .block(Block::default())
                        .style(Style::default().fg(Color::White).bg(Color::Black));
                    f.render_widget(text, content_area);
                } else {
                    let empty = Paragraph::new("No text content loaded")
                        .style(Style::default().fg(Color::DarkGray))
                        .block(Block::default());
                    f.render_widget(empty, content_area);
                }
            }
            DisplayMode::Image => {
                if let Some(ref mut image_state) = app.image_state {
                    // Use Scale mode to resize the image to fill the available space
                    // The widget will maintain aspect ratio and center it automatically
                    let image_widget = StatefulImage::default()
                        .resize(Resize::Scale(None));
                    f.render_stateful_widget(image_widget, content_area, image_state);
                } else {
                    let empty = Paragraph::new("No image loaded")
                        .style(Style::default().fg(Color::DarkGray))
                        .block(Block::default());
                    f.render_widget(empty, content_area);
                }
            }
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
        "  [← / →] Page  [↑/↓] Sub-page  [0-9] Jump  [v] Toggle view  [q] Quit  [c] Clear cache".to_string()
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

