use druid::kurbo::Line;
use druid::piet::StrokeStyle;
use druid::{
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget,
};

use crate::HantekState;

pub struct ScopeGraph;

impl Widget<HantekState> for ScopeGraph {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut HantekState, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &HantekState,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: &HantekState,
        _data: &HantekState,
        _env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &HantekState,
        _env: &Env,
    ) -> Size {
        if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            let size = Size::new(100.0, 100.0);
            bc.constrain(size)
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &HantekState, _env: &Env) {
        let num_samples_0 = match data
            .capture
            .as_ref()
            .unwrap_or(&Vec::<u8>::with_capacity(0))
            .len()
        {
            0 => 1024,
            anything => anything,
        };

        let mut channels: Vec<usize> = Vec::with_capacity(2);
        let mut num_channels = 0usize;
        let has_ch1 = data.cfg.enabled_channels[&1].unwrap();
        let has_ch2 = data.cfg.enabled_channels[&2].unwrap();
        if has_ch1 {
            num_channels += 1;
            channels.push(1);
        }
        if has_ch2 {
            num_channels += 1;
            channels.push(2);
        }
        if num_channels == 0 {
            num_channels = 1;
        }

        let num_samples: usize = num_channels * num_samples_0;
        let num_sectors: usize = num_samples / 100;
        let num_sectors_f: f64 = num_samples as f64 / 100.0f64;

        let size = ctx.size();
        let rect = size.to_rect();
        let stroke_color = Color::rgba(255., 255., 255., 0.5);
        let dashed = StrokeStyle::new().dash_pattern(&[4.0, 4.0]);
        let width = size.width;
        let height = size.height;

        let ch1_stroke_color = Color::rgba(255., 0., 0., 0.7);
        let ch2_stroke_color = Color::rgba(0., 255., 0., 0.7);

        ctx.fill(rect, &Color::BLACK);

        ctx.paint_with_z_index(1, move |ctx| {
            for i in 1..num_sectors {
                let x = (i as f64) * width / num_sectors_f;
                let path = Line::new((x, 0.0), (x, height));
                ctx.stroke_styled(path, &stroke_color, 0.5, &dashed);
            }

            for i in 1..=8 {
                let y = (i as f64) * height / 8.0;
                let path = Line::new((0.0, y), (width, y));
                ctx.stroke_styled(path, &stroke_color, 0.5, &dashed);
            }
        });

        let capture = data
            .capture
            .as_ref()
            .unwrap_or(&Vec::with_capacity(0))
            .clone();

        ctx.paint_with_z_index(1, move |ctx| {
            for c in channels {
                let ch_stroke_color = match c {
                    1 => &ch1_stroke_color,
                    2 => &ch2_stroke_color,
                    _ => panic!("unexpected channel number: {}", c),
                };

                for i in 1..capture.len() {
                    if i * 2 + c >= capture.len() {
                        break;
                    }
                    let from_x = ((i - 1) as f64) * width / (num_samples_0 as f64);
                    let to_x = (i as f64) * width / (num_samples_0 as f64);
                    let y = capture[i * 2 + c] as i32;
                    let y = ((y - 29) as f64) * height / 202.;
                    let path = Line::new((from_x, y), (to_x, y));
                    ctx.stroke(path, ch_stroke_color, 2.);
                }
            }
        });
    }
}
