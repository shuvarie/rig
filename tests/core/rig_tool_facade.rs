//! Implementing `Tool` by hand.
//!
//! The `#[rig_tool]` attribute is gone; this is the impl it generated, written
//! out. The schema still comes from the argument type's `JsonSchema`, so the
//! parameters the model sees and the struct the arguments deserialize into
//! cannot drift.

use rig::tool::{Tool, ToolContext, ToolExecutionError, tool_definition};

#[derive(serde::Deserialize, rig::schemars::JsonSchema)]
struct FacadeCoordinates {
    /// Latitude
    lat: f64,
    /// Longitude
    lng: f64,
}

/// Arguments of the `facade_find_nearby` tool.
#[derive(serde::Deserialize, rig::schemars::JsonSchema)]
struct FacadeFindNearbyArgs {
    /// Location to search from
    location: FacadeCoordinates,
}

/// Find nearby places
struct FacadeFindNearby;

impl Tool for FacadeFindNearby {
    const NAME: &'static str = "facade_find_nearby";
    type Error = ToolExecutionError;
    type Args = FacadeFindNearbyArgs;
    type Output = String;

    fn description(&self) -> String {
        "Find nearby places".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(rig::schemars::schema_for!(FacadeFindNearbyArgs))
            .expect("the argument schema serializes")
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(format!("{},{}", args.location.lat, args.location.lng))
    }
}

#[tokio::test]
async fn a_hand_written_tool_advertises_its_schema() {
    let definition = tool_definition(&FacadeFindNearby);
    let schema = serde_json::to_string(&definition.parameters).unwrap();

    assert!(schema.contains("lat"), "expected lat in schema: {schema}");
    assert!(schema.contains("lng"), "expected lng in schema: {schema}");
    assert!(
        schema.contains("Latitude"),
        "expected Latitude description in schema: {schema}"
    );
    assert_eq!(definition.description, "Find nearby places");
}
