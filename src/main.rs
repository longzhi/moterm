mod clipboard;
mod color;
mod font;
mod input;
mod pty;
mod renderer;
mod terminal;
mod vte_handler;

use std::sync::{Arc, Mutex};

use renderer::Renderer;
use terminal::Terminal;
use vte_handler::VteHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{
    ElementState, Event, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::WindowBuilder;

use crate::pty::{PtyEvent, PtyHandle};

#[derive(Debug, Clone)]
enum AppEvent {
    PtyOutput(Vec<u8>),
    PtyExit,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("moterm 启动失败: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (font, font_path) = font::load_monospace_font()?;
    eprintln!("使用字体: {}", font_path.display());

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("moterm")
        .with_inner_size(LogicalSize::new(960.0, 600.0))
        .with_resizable(true)
        .build(&event_loop)
        .map_err(|e| format!("创建窗口失败: {e}"))?;

    let mut renderer = Renderer::new(font, 16.0);
    let size = window.inner_size();
    let (cols, rows) = renderer.grid_size_for_pixels(size.width as usize, size.height as usize);
    let mut term = Terminal::new(cols, rows);

    let proxy = event_loop.create_proxy();
    let pty = PtyHandle::spawn(cols as u16, rows as u16, move |ev| {
        let _ = match ev {
            PtyEvent::Output(data) => proxy.send_event(AppEvent::PtyOutput(data)),
            PtyEvent::Exit => proxy.send_event(AppEvent::PtyExit),
        };
    })?;

    let mut parser = vte::Parser::new();
    let mut dirty = true;
    let mut modifiers = ModifiersState::empty();
    let mut mouse_pos = PhysicalPosition::new(0.0f64, 0.0f64);
    let mut selecting = false;

    let context = unsafe { softbuffer::Context::new(&window) }
        .map_err(|e| format!("softbuffer context 创建失败: {e}"))?;
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }
        .map_err(|e| format!("softbuffer surface 创建失败: {e}"))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(AppEvent::PtyOutput(data)) => {
                {
                    let mut performer = VteHandler::new(&mut term);
                    for b in data {
                        parser.advance(&mut performer, b);
                    }
                }
                dirty = true;
                window.request_redraw();
            }
            Event::UserEvent(AppEvent::PtyExit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(new_size) => {
                    let (cols, rows) = renderer
                        .grid_size_for_pixels(new_size.width as usize, new_size.height as usize);
                    term.resize(cols, rows);
                    if let Ok(pty) = pty.lock() {
                        pty.resize(cols as u16, rows as u16);
                    }
                    dirty = true;
                    window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    let (cols, rows) = renderer.grid_size_for_pixels(
                        new_inner_size.width as usize,
                        new_inner_size.height as usize,
                    );
                    term.resize(cols, rows);
                    if let Ok(pty) = pty.lock() {
                        pty.resize(cols as u16, rows as u16);
                    }
                    dirty = true;
                    window.request_redraw();
                }
                WindowEvent::ModifiersChanged(m) => {
                    modifiers = m;
                }
                WindowEvent::ReceivedCharacter(ch) => {
                    if let Some(bytes) = input::map_received_char(ch, modifiers) {
                        write_pty(&pty, &bytes);
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state != ElementState::Pressed {
                        return;
                    }
                    if let Some(key) = input.virtual_keycode {
                        if modifiers.logo()
                            && key == winit::event::VirtualKeyCode::C
                            && term.selection_non_empty()
                        {
                            let text = term.selection_text_or_empty();
                            if let Err(e) = clipboard::copy_to_clipboard(&text) {
                                eprintln!("复制失败: {e}");
                            }
                            return;
                        }

                        match key {
                            winit::event::VirtualKeyCode::PageUp if modifiers.shift() => {
                                term.scroll_view_page(1);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::PageDown if modifiers.shift() => {
                                term.scroll_view_page(-1);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::Home if modifiers.shift() => {
                                term.set_view_scroll(term.max_view_scroll() as isize);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::End if modifiers.shift() => {
                                term.scroll_view_to_bottom();
                                dirty = true;
                                window.request_redraw();
                            }
                            _ => {
                                if let Some(bytes) = input::map_special_key(key, modifiers) {
                                    write_pty(&pty, &bytes);
                                }
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    mouse_pos = position;
                    if selecting {
                        if let Some((view_row, col)) = pixel_to_cell(&renderer, &window, mouse_pos)
                        {
                            term.set_selection_focus_from_view(view_row, col);
                            dirty = true;
                            window.request_redraw();
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => match state {
                    ElementState::Pressed => {
                        selecting = true;
                        if let Some((view_row, col)) = pixel_to_cell(&renderer, &window, mouse_pos)
                        {
                            term.start_selection_from_view(view_row, col);
                            dirty = true;
                            window.request_redraw();
                        }
                    }
                    ElementState::Released => {
                        selecting = false;
                    }
                },
                WindowEvent::MouseWheel { delta, .. } => {
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y.round() as isize,
                        MouseScrollDelta::PixelDelta(p) => {
                            (p.y / renderer.atlas.cell_height as f64).round() as isize
                        }
                    };
                    if lines != 0 {
                        term.set_view_scroll(-lines);
                        dirty = true;
                        window.request_redraw();
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                if !dirty {
                    return;
                }
                let size = window.inner_size();
                let (w_nz, h_nz) = renderer::Renderer::nonzero_dims(size.width, size.height);
                if let Err(e) = surface.resize(w_nz, h_nz) {
                    eprintln!("surface resize 失败: {e}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                renderer.render(&term, size.width as usize, size.height as usize);

                match surface.buffer_mut() {
                    Ok(mut buffer) => {
                        if buffer.len() == renderer.canvas.pixels.len() {
                            buffer.copy_from_slice(&renderer.canvas.pixels);
                        } else {
                            for (dst, src) in buffer
                                .iter_mut()
                                .zip(renderer.canvas.pixels.iter().copied())
                            {
                                *dst = src;
                            }
                        }
                        if let Err(e) = buffer.present() {
                            eprintln!("present 失败: {e}");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                    Err(e) => {
                        eprintln!("获取绘制缓冲区失败: {e}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
                dirty = false;
            }
            Event::MainEventsCleared => {}
            _ => {}
        }
    });
}

fn pixel_to_cell(
    renderer: &Renderer,
    window: &winit::window::Window,
    pos: PhysicalPosition<f64>,
) -> Option<(usize, usize)> {
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let x = pos.x.max(0.0) as usize;
    let y = pos.y.max(0.0) as usize;
    if x < renderer.padding_x || y < renderer.padding_y {
        return None;
    }
    let gx = x - renderer.padding_x;
    let gy = y - renderer.padding_y;
    Some((
        gy / renderer.atlas.cell_height,
        gx / renderer.atlas.cell_width,
    ))
}

fn write_pty(pty: &Arc<Mutex<PtyHandle>>, bytes: &[u8]) {
    if let Ok(pty) = pty.lock() {
        let _ = pty.write(bytes);
    }
}
