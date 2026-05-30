pub mod readable;
pub mod strategy;

use boa_engine::{Context, JsResult};

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    strategy::register_globals(context)?;
    readable::register_globals(context)?;
    Ok(())
}
