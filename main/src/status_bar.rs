use crate::{display::LogicalDisplayId, sys_info::SysInfo};
use core_graphics::Bounds;
use foundation::{Application, Colour, Label, Window};

pub struct StatusBar {
    logical_ids: Vec<LogicalDisplayId>,
    window: Window,
    width: f64,
}

impl StatusBar {
    pub const HEIGHT: f64 = 25.0;
    const ID_START_X: f64 = 0.0;
    const ID_WIDTH: f64 = 20.0;

    pub fn new(logical_ids: Vec<LogicalDisplayId>, bounds: Bounds) -> Self {
        let mut logical_ids = logical_ids;
        logical_ids.sort();

        let _application = Application::default();

        let main_display_bounds = core_graphics::Display::main_display_bounds();
        let window_bottom_left = main_display_bounds.height - (bounds.y + bounds.height);
        let window_bounds = Bounds {
            x: bounds.x,
            y: window_bottom_left,
            height: Self::HEIGHT,
            width: bounds.width,
        };

        let mut window = Window::new(window_bounds);
        window.set_background_colour(Colour::Black);

        let sys_info = SysInfo::new();

        let ipv4_label_bounds = Bounds {
            x: bounds.width - 100.0,
            y: 0.0,
            height: 25.0,
            width: 100.0,
        };

        let ipv6_label_bounds = Bounds {
            x: bounds.width - 150.0,
            y: 0.0,
            height: Self::HEIGHT,
            width: 50.0,
        };

        let ipv4_label = Self::ipv4_label(sys_info.ipv4, ipv4_label_bounds);
        let ipv6_label = Self::ipv6_label(sys_info.ipv6, ipv6_label_bounds);

        for (i, id) in logical_ids.iter().enumerate() {
            let display_id_bounds = Bounds {
                x: Self::ID_START_X + (i as f64 * Self::ID_WIDTH),
                y: 0.0,
                height: Self::HEIGHT,
                width: Self::ID_WIDTH,
            };

            let colour = Colour::White;

            // TODO: handle i3-ism of 0 == 10?

            let id_label = Self::id_label(*id, display_id_bounds, colour);
            window.add_element_to_content_view(id_label);
        }

        window.add_element_to_content_view(ipv4_label);
        window.add_element_to_content_view(ipv6_label);

        Self {
            logical_ids,
            window,
            width: bounds.width,
        }
    }

    pub fn remove_logical_id(&mut self, logical_id: LogicalDisplayId) {
        self.logical_ids.retain(|&id| id != logical_id);
    }

    pub fn add_logical_id(&mut self, logical_id: LogicalDisplayId) {
        self.logical_ids.push(logical_id);
        self.logical_ids.sort();
    }

    pub fn close(&mut self) {
        self.window.close();
    }

    pub fn draw(&mut self, active_id: LogicalDisplayId) {
        self.window.clear_content_view();

        for (i, id) in self.logical_ids.iter().enumerate() {
            let display_id_bounds = Bounds {
                x: Self::ID_START_X + (i as f64 * Self::ID_WIDTH),
                y: 0.0,
                height: Self::HEIGHT,
                width: Self::ID_WIDTH,
            };

            let colour = if *id == active_id {
                Colour::Blue
            } else {
                Colour::White
            };

            let id_label = Self::id_label(*id, display_id_bounds, colour);
            self.window.add_element_to_content_view(id_label);
        }

        // IP labels
        let sys_info = SysInfo::new();

        let ipv4_label_bounds = Bounds {
            x: self.width - 100.0,
            y: 0.0,
            height: Self::HEIGHT,
            width: 100.0,
        };

        let ipv6_label_bounds = Bounds {
            x: self.width - 150.0,
            y: 0.0,
            height: Self::HEIGHT,
            width: 50.0,
        };

        let ipv4_label = Self::ipv4_label(sys_info.ipv4, ipv4_label_bounds);
        let ipv6_label = Self::ipv6_label(sys_info.ipv6, ipv6_label_bounds);

        self.window.add_element_to_content_view(ipv4_label);
        self.window.add_element_to_content_view(ipv6_label);

        self.window.display();
    }

    fn ipv4_label(ipv4_addr: Option<String>, bounds: Bounds) -> Label {
        let ipv4_addr_colour = if ipv4_addr.is_some() {
            Colour::Green
        } else {
            Colour::Red
        };

        Label::new(
            bounds,
            ipv4_addr.unwrap_or("W: down".to_string()),
            ipv4_addr_colour,
        )
    }

    fn ipv6_label(ipv6_addr: Option<String>, bounds: Bounds) -> Label {
        let ipv6_addr_colour = if ipv6_addr.is_some() {
            Colour::Green
        } else {
            Colour::Red
        };

        Label::new(
            bounds,
            ipv6_addr.unwrap_or("no IPv6".to_string()),
            ipv6_addr_colour,
        )
    }

    fn id_label(display_id: LogicalDisplayId, bounds: Bounds, colour: Colour) -> Label {
        Label::new(bounds, display_id.to_string(), colour)
    }
}
