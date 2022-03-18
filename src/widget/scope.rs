use crate::HantekState;
use druid::kurbo::BezPath;
use druid::{
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget,
};

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

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &HantekState, _env: &Env) {
        let size = ctx.size();
        let rect = size.to_rect();
        ctx.fill(rect, &Color::WHITE);

        ctx.paint_with_z_index(1, move |ctx| {
            let mut path = BezPath::new();
            path.move_to((0.0, size.height));
            path.quad_to((40.0, 50.0), (size.width, 0.0));
            let stroke_color = Color::rgb8(128, 0, 0);
            ctx.stroke(path, &stroke_color, 5.0);
        })
    }
}
