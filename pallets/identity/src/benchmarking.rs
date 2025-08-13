//! Benchmarking setup for pallet-identity
//!
//! This module contains comprehensive benchmarks for the Identity pallet,
//! designed to showcase various benchmarking patterns and complexities:
//!
//! 1. **Linear complexity** - `set_identity` scales with identity data size O(n)
//! 2. **Logarithmic complexity** - `provide_judgement` uses binary search O(log n)  
//! 3. **Linear cleanup** - `clear_identity` scales with number of judgements O(j)
//! 4. **Economic operations** - Currency operations (reserve, unreserve)
//! 5. **Vector operations** - Sorted insertion and binary search in bounded collections
//! 6. **Storage operations** - Multiple storage interactions with proper state management
//!
//! ## Learning Objectives
//!
//! - Understanding different complexity patterns (linear vs logarithmic)
//! - Using multiple complexity parameters (b for bytes, j for judgements)
//! - Measuring worst-case execution paths for different algorithms
//! - Binary search benchmarking with sorted data structures
//! - Vector operations with bounded collections
//! - Verifying benchmark correctness with comprehensive assertions

#![cfg(feature = "runtime-benchmarks")]
use super::*;

use crate::{Pallet as Identity, Config, IdentityInfo, Judgement};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{Currency, Get, ReservableCurrency},
	BoundedVec,
};
use frame_system::RawOrigin;
use sp_runtime::traits::Zero;
use sp_std::vec;

/// Create a reasonable identity info for benchmarking
/// This helper demonstrates how to set up test data for benchmarks
fn create_identity_info(bytes: u32) -> IdentityInfo {
	let data = vec![b'X'; bytes.min(MAX_FIELD_LENGTH) as usize];
	let bounded_data = BoundedVec::try_from(data).unwrap_or_default();
	
	IdentityInfo {
		display: bounded_data.clone(),
		legal: bounded_data.clone(),
		web: bounded_data.clone(),
		email: bounded_data,
	}
}

/// Fund an account with enough balance for benchmarking operations
/// This helper ensures accounts have sufficient funds for deposits
fn fund_account<T: Config>(account: &T::AccountId) {
	let min_balance = T::Currency::minimum_balance();
	let deposit_required = T::BasicDeposit::get() + T::ByteDeposit::get() * 1000u32.into();
	let total = min_balance + deposit_required;
	T::Currency::make_free_balance_be(account, total);
}

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Benchmark: set_identity
	/// 
	/// Complexity: Linear in the number of bytes of identity information (b)
	/// This benchmark demonstrates:
	/// - Linear complexity with respect to data size
	/// - Economic operations (currency reservation)
	/// - Storage operations (conditional insertion/update)
	/// - Event emission
	#[benchmark]
	fn set_identity(
		// Parameter 'b' represents the number of bytes in the identity info
		// This creates a linear relationship between input size and execution time
		b: Linear<1, { MAX_FIELD_LENGTH }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		fund_account::<T>(&caller);
		
		let identity_info = create_identity_info(b);
		let expected_deposit = T::BasicDeposit::get() + 
			T::ByteDeposit::get() * u32::from(identity_info.encoded_size()).into();

		#[extrinsic_call]
		set_identity(
			RawOrigin::Signed(caller.clone()),
			identity_info.display.clone(),
			identity_info.legal.clone(),
			identity_info.web.clone(),
			identity_info.email.clone(),
		);

		// Verify the benchmark worked correctly
		let registration = IdentityOf::<T>::get(&caller).unwrap();
		assert_eq!(registration.info, identity_info);
		assert_eq!(registration.deposit, expected_deposit);
		assert_eq!(registration.judgements.len(), 0);
		assert_eq!(T::Currency::reserved_balance(&caller), expected_deposit);
	}

	/// Benchmark: set_identity_update
	/// 
	/// This benchmark tests the update path when an identity already exists
	/// It demonstrates conditional logic benchmarking - measuring the "update" case
	/// vs the "insert" case measured in set_identity above
	#[benchmark]
	fn set_identity_update(
		b: Linear<1, { MAX_FIELD_LENGTH }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		fund_account::<T>(&caller);
		
		// Pre-condition: set an initial identity
		let initial_info = create_identity_info(b / 2);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(caller.clone()).into(),
			initial_info.display,
			initial_info.legal,
			initial_info.web,
			initial_info.email,
		);
		
		let new_identity_info = create_identity_info(b);

		#[extrinsic_call]
		set_identity(
			RawOrigin::Signed(caller.clone()),
			new_identity_info.display.clone(),
			new_identity_info.legal.clone(),
			new_identity_info.web.clone(),
			new_identity_info.email.clone(),
		);

		// Verify the update worked
		let registration = IdentityOf::<T>::get(&caller).unwrap();
		assert_eq!(registration.info, new_identity_info);
	}

	/// Benchmark: clear_identity
	/// 
	/// Complexity: Linear in the number of judgements (j)
	/// This benchmark demonstrates:
	/// - Storage cleanup operations with linear complexity
	/// - Economic operations (unreserving currency)
	/// - Vector cleanup proportional to number of judgements
	#[benchmark]
	fn clear_identity(
		j: Linear<0, { T::MaxJudgements::get() }>,  // Number of judgements
	) {
		let caller: T::AccountId = whitelisted_caller();
		fund_account::<T>(&caller);
		
		// Pre-condition: set up identity
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(caller.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);

		// Add judgements to create linear complexity in cleanup
		for i in 0..j {
			IdentityOf::<T>::mutate(&caller, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((i, Judgement::Reasonable));
				}
			});
		}

		let _deposit_before = T::Currency::reserved_balance(&caller);

		#[extrinsic_call]
		clear_identity(RawOrigin::Signed(caller.clone()));

		// Verify storage was cleared and deposit returned
		assert!(IdentityOf::<T>::get(&caller).is_none());
		assert_eq!(T::Currency::reserved_balance(&caller), Zero::zero());
		assert_eq!(T::Currency::free_balance(&caller), 
			T::Currency::total_balance(&caller));
	}


	/// Benchmark: provide_judgement
	/// 
	/// This benchmark tests providing a judgement on an identity with existing judgements
	/// Complexity: Logarithmic in the number of existing judgements (j) for binary search
	/// This demonstrates logarithmic complexity O(log n) operations
	#[benchmark]
	fn provide_judgement(
		j: Linear<0, { T::MaxJudgements::get() - 1 }>,  // Max existing judgements so we can add one more
	) {
		let target: T::AccountId = account("target", 0, 0);
		fund_account::<T>(&target);
		
		// Pre-condition: set up identity
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(target.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);

		// Add existing judgements to create worst-case binary search scenario
		// We'll add judgements with IDs 1, 3, 5, 7, ... (odd numbers)
		// Then insert with ID 0 to test binary search at the beginning
		for i in 0..j {
			let judgement_id = (i * 2) + 1; // Creates IDs: 1, 3, 5, 7, ...
			IdentityOf::<T>::mutate(&target, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((judgement_id, Judgement::Reasonable));
				}
			});
		}

		let new_judgement_id = 0u32; // This will be inserted at position 0
		let judgement_type = 2u8; // KnownGood

		#[extrinsic_call]
		provide_judgement(RawOrigin::Root, new_judgement_id, target.clone(), judgement_type);

		// Verify judgement was provided and inserted correctly
		let registration = IdentityOf::<T>::get(&target).unwrap();
		assert_eq!(registration.judgements.len(), (j + 1) as usize);
		assert_eq!(registration.judgements[0], (new_judgement_id, Judgement::KnownGood));
		// Verify ordering is maintained
		for i in 1..registration.judgements.len() {
			assert!(registration.judgements[i-1].0 < registration.judgements[i].0);
		}
	}

	impl_benchmark_test_suite!(Identity, crate::mock::new_test_ext(), crate::mock::Test);
}