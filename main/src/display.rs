use crate::window::Window;
use core_graphics::Bounds;

#[derive(Debug, Default)]
enum Direction {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug)]
enum Container<'a> {
    Leaf(Window<'a>),
    Split {
        direction: Direction,
        children: Vec<Container<'a>>,
    },
}

#[derive(Debug)]
pub(crate) struct Display<'a> {
    id: u64,
    bounds: Bounds,
    root: Container<'a>,
}

impl<'a> Display<'a> {
    pub(crate) fn try_new(display: &'a core_graphics::Display) -> crate::Result<Self> {
        let container = match display.windows.len() {
            1 => {
                let mut w = Window::try_new(&display.windows[0], display.bounds)?;
                w.init()?;
                Container::Leaf(w)
            }
            n => {
                let widths = split_n(display.bounds.width, n);
                let xs = xs_from_widths(display.bounds.x, &widths);
                let bounds: Vec<Bounds> = widths
                    .iter()
                    .zip(xs.iter())
                    .map(|(&width, &x)| Bounds {
                        width,
                        x,
                        ..display.bounds
                    })
                    .collect();

                let children = (0..n)
                    .map(|i| match Window::try_new(&display.windows[i], bounds[i]) {
                        Ok(mut window) => {
                            window.init()?;
                            Ok(window)
                        }
                        Err(e) => Err(e),
                    })
                    .collect::<crate::Result<Vec<Window>>>()?;

                Container::Split {
                    direction: Direction::default(),
                    children: children.into_iter().map(Container::Leaf).collect(),
                }
            }
        };

        Ok(Self {
            // TODO: get id
            id: 0,
            bounds: display.bounds,
            root: container,
        })
    }
}

fn split_n(total: f64, n: usize) -> Vec<f64> {
    let base = total / n as f64;
    let remainder = total % n as f64;

    (0..n)
        .map(|i| {
            if (i as f64) < remainder {
                base + 1.0
            } else {
                base
            }
        })
        .collect()
}

fn xs_from_widths(start: f64, widths: &[f64]) -> Vec<f64> {
    let mut xs = Vec::with_capacity(widths.len());
    xs.push(start);
    for w in widths.iter().skip(1) {
        xs.push(xs[xs.len() - 1] + w);
    }

    xs
}
