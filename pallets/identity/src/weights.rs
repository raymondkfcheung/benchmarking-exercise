use frame_support::weights::Weight;

/// Weight functions needed for pallet_identity.
pub trait WeightInfo {
	fn set_identity(b: u32) -> Weight;
	fn set_identity_update(b: u32, j: u32) -> Weight;
	fn provide_judgement_inline(j: u32) -> Weight;
	fn provide_judgement_double_map() -> Weight;
	fn clear_identity_inline_usage(j: u32) -> Weight;
	fn clear_identity_double_map_usage(j: u32) -> Weight;
}

/// Dummy weight implementation for unit type
impl WeightInfo for () {
	fn set_identity(_b: u32) -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn set_identity_update(_b: u32, _j: u32) -> Weight {
		Weight::from_parts(20_000, 0)
	}
	fn provide_judgement_inline(_j: u32) -> Weight {
		Weight::from_parts(15_000, 0)
	}
	fn provide_judgement_double_map() -> Weight {
		Weight::from_parts(12_000, 0)
	}
	fn clear_identity_inline_usage(_j: u32) -> Weight {
		Weight::from_parts(8_000, 0)
	}
	fn clear_identity_double_map_usage(_j: u32) -> Weight {
		Weight::from_parts(25_000, 0)
	}
}
