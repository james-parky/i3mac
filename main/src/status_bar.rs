use crate::display;
use crate::display::logical;
use crate::sys_info::SysInfo;
use core_graphics::Bounds;
use foundation::{Application, Colour, IdLabel, Label, Window};

// Ideas:
//  - Transparent like MacOS status bar at the top of the screen
//  - White text on darker wallpapers, light text on darker
//  - All items become slightly transparent when the display is not focussed
//
//  - Logical display ids, distinguish active using filled in box, outlines for
//    not currently focussed
//  - Wifi strength and IPaddr, v4 + v6
//  - Ethernet connectivity,
//  - Current disk usage
//  - Current total cpu load
//  - Current mem usage / total
//
//  - kqueue events to see if wifi etc has been changed and timer fd for cpu/mem
pub struct StatusBar {
    logical_ids: Vec<logical::Id>,
    window: Window,
    width: f64,
}

impl StatusBar {
    pub const HEIGHT: f64 = 25.0;
    const ID_START_X: f64 = 0.0;
    const ID_WIDTH: f64 = 25.0;
    const ACTIVE_OPACITY: f64 = 1.0;
    const INACTIVE_OPACITY: f64 = 0.6;

    pub fn new(logical_ids: Vec<logical::Id>, bounds: Bounds, background: Colour) -> Self {
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
        window.set_background_colour(background);

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
                x: Self::ID_START_X + (i as f64 * Self::ID_WIDTH) + 2.5,
                y: 2.5,
                height: Self::HEIGHT - 5.0,
                width: Self::ID_WIDTH - 5.0,
            };

            let colour = Colour::White;

            // TODO: handle i3-ism of 0 == 10?

            let id_label = IdLabel::new_inactive(display_id_bounds, (*id).to_string());
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

    pub fn set_active(&mut self, active: bool) {
        let opacity = if active {
            Self::ACTIVE_OPACITY
        } else {
            Self::INACTIVE_OPACITY
        };

        self.window.set_opacity(opacity);
    }

    pub fn remove_logical_id(&mut self, logical_id: logical::Id) {
        self.logical_ids.retain(|&id| id != logical_id);
    }

    pub fn add_logical_id(&mut self, logical_id: logical::Id) {
        if !self.logical_ids.contains(&logical_id) {
            self.logical_ids.push(logical_id);
            self.logical_ids.sort();
        }
    }

    pub fn close(&mut self) {
        self.window.close();
    }

    pub fn draw(&mut self, active_id: logical::Id) {
        self.set_active(self.logical_ids.contains(&active_id));
        self.window.clear_content_view();

        for (i, id) in self.logical_ids.iter().enumerate() {
            let display_id_bounds = Bounds {
                x: Self::ID_START_X + (i as f64 * Self::ID_WIDTH) + 2.5,
                y: 2.5,
                height: Self::HEIGHT - 5.0,
                width: Self::ID_WIDTH - 5.0,
            };

            let id_label = if *id == active_id {
                IdLabel::new_active(display_id_bounds, (*id).to_string())
            } else {
                IdLabel::new_inactive(display_id_bounds, (*id).to_string())
            };
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
        Label::new(
            bounds,
            ipv4_addr.unwrap_or("W: down".to_string()),
            Colour::White,
        )
    }

    fn ipv6_label(ipv6_addr: Option<String>, bounds: Bounds) -> Label {
        Label::new(
            bounds,
            ipv6_addr.unwrap_or("no IPv6".to_string()),
            Colour::White,
        )
    }
}
