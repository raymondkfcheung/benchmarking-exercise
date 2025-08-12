//! Benchmarking setup for pallet-identity
//!
//! This module contains comprehensive benchmarks for the Identity pallet,
//! designed to showcase various benchmarking patterns and complexities:
//!
//! 1. **Simple operations** - Basic storage reads/writes
//! 2. **Linear complexity** - Operations that scale with input size
//! 3. **Database operations** - Multiple storage interactions
//! 4. **Economic operations** - Currency operations (reserve, unreserve, slash)
//! 5. **Conditional logic** - Different execution paths
//! 6. **Vector operations** - Working with bounded collections
//!
//! ## Learning Objectives
//!
//! - Understanding benchmark setup and teardown
//! - Using linear complexity parameters (r, s, etc.)
//! - Measuring worst-case execution paths
//! - Handling storage pre-conditions and post-conditions
//! - Testing economic operations
//! - Verifying benchmark correctness with assertions

#![cfg(feature = "runtime-benchmarks")]
use super::*;

use crate::{Pallet as Identity, Config, IdentityInfo, Judgement, RegistrarInfo};
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
	/// - Storage cleanup operations
	/// - Economic operations (unreserving currency)
	/// - Linear complexity based on number of associated items
	#[benchmark]
	fn clear_identity(
		j: Linear<0, { T::MaxRegistrars::get() }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		fund_account::<T>(&caller);
		
		// Pre-condition: set up identity with judgements
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(caller.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);
		
		// Add registrars and judgements for linear complexity
		for i in 0..j {
			let registrar: T::AccountId = account("registrar", i, 0);
			fund_account::<T>(&registrar);
			let _ = Identity::<T>::add_registrar(RawOrigin::Root.into(), registrar.clone());
			
			// Add a judgement to create linear complexity
			IdentityOf::<T>::mutate(&caller, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((i, Judgement::Reasonable));
				}
			});
		}

		let deposit_before = T::Currency::reserved_balance(&caller);

		#[extrinsic_call]
		clear_identity(RawOrigin::Signed(caller.clone()));

		// Verify storage was cleared and deposit returned
		assert!(IdentityOf::<T>::get(&caller).is_none());
		assert_eq!(T::Currency::reserved_balance(&caller), Zero::zero());
		assert_eq!(T::Currency::free_balance(&caller), 
			T::Currency::total_balance(&caller));
	}

	/// Benchmark: request_judgement
	/// 
	/// Complexity: Logarithmic in the number of existing judgements (j) for binary search
	/// This benchmark demonstrates:
	/// - Logarithmic complexity (binary search in sorted vector)
	/// - Economic operations (additional currency reservation)
	/// - Vector manipulation (sorted insertion)
	#[benchmark]
	fn request_judgement(
		j: Linear<0, { T::MaxRegistrars::get() - 1 }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		fund_account::<T>(&caller);
		
		// Pre-condition: set identity
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(caller.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);
		
		// Add registrars for complexity
		let mut registrar_accounts = vec![];
		for i in 0..=j {
			let registrar: T::AccountId = account("registrar", i, 0);
			fund_account::<T>(&registrar);
			registrar_accounts.push(registrar.clone());
			let _ = Identity::<T>::add_registrar(RawOrigin::Root.into(), registrar);
		}
		
		// Add existing judgements to create worst-case binary search scenario
		// We add judgements for registrars 1..j, then request for registrar 0
		// This creates a scenario where binary search has to work
		for i in 1..=j {
			IdentityOf::<T>::mutate(&caller, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((i, Judgement::Reasonable));
				}
			});
		}

		let reg_index = 0u32;
		let max_fee = T::Currency::minimum_balance() * 100u32.into();

		#[extrinsic_call]
		request_judgement(RawOrigin::Signed(caller.clone()), reg_index, max_fee);

		// Verify judgement was requested
		let registration = IdentityOf::<T>::get(&caller).unwrap();
		assert!(registration.judgements.iter().any(|(idx, _)| *idx == reg_index));
	}

	/// Benchmark: provide_judgement
	/// 
	/// This benchmark tests the registrar providing a judgement
	/// It demonstrates role-based operations and payment flows
	#[benchmark]
	fn provide_judgement(
		j: Linear<1, { T::MaxRegistrars::get() }>,
	) {
		let target: T::AccountId = account("target", 0, 0);
		let registrar: T::AccountId = account("registrar", 0, 0);
		fund_account::<T>(&target);
		fund_account::<T>(&registrar);
		
		// Pre-condition: set up identity, registrar, and judgement request
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(target.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);
		
		let _ = Identity::<T>::add_registrar(RawOrigin::Root.into(), registrar.clone());
		let _ = Identity::<T>::request_judgement(
			RawOrigin::Signed(target.clone()).into(),
			0,
			T::Currency::minimum_balance() * 100u32.into()
		);
		
		// Add additional judgements for complexity
		for i in 1..j {
			let other_registrar: T::AccountId = account("other_reg", i, 0);
			fund_account::<T>(&other_registrar);
			let _ = Identity::<T>::add_registrar(RawOrigin::Root.into(), other_registrar.clone());
			
			IdentityOf::<T>::mutate(&target, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((i, Judgement::Reasonable));
				}
			});
		}

		let reg_index = 0u32;
		let judgement_type = 2u8; // KnownGood

		#[extrinsic_call]
		provide_judgement(RawOrigin::Signed(registrar.clone()), reg_index, target.clone(), judgement_type);

		// Verify judgement was provided
		let registration = IdentityOf::<T>::get(&target).unwrap();
		assert_eq!(
			registration.judgements.iter().find(|(idx, _)| *idx == reg_index).unwrap().1,
			Judgement::KnownGood
		);
	}

	/// Benchmark: kill_identity
	/// 
	/// This benchmark tests the force removal of an identity
	/// It demonstrates admin operations and slashing
	#[benchmark]
	fn kill_identity(
		j: Linear<0, { T::MaxRegistrars::get() }>,
	) {
		let target: T::AccountId = account("target", 0, 0);
		fund_account::<T>(&target);
		
		// Pre-condition: set up identity with judgements
		let identity_info = create_identity_info(10);
		let _ = Identity::<T>::set_identity(
			RawOrigin::Signed(target.clone()).into(),
			identity_info.display,
			identity_info.legal,
			identity_info.web,
			identity_info.email,
		);
		
		// Add judgements for complexity
		for i in 0..j {
			let registrar: T::AccountId = account("registrar", i, 0);
			fund_account::<T>(&registrar);
			let _ = Identity::<T>::add_registrar(RawOrigin::Root.into(), registrar);
			
			IdentityOf::<T>::mutate(&target, |maybe_reg| {
				if let Some(ref mut reg) = maybe_reg {
					let _ = reg.judgements.try_push((i, Judgement::Reasonable));
				}
			});
		}

		let deposit_before = T::Currency::reserved_balance(&target);
		assert!(!deposit_before.is_zero());

		#[extrinsic_call]
		kill_identity(RawOrigin::Root, target.clone());

		// Verify identity was killed and deposit slashed
		assert!(IdentityOf::<T>::get(&target).is_none());
		// Note: After slashing, reserved balance should be reduced
	}

	/// Benchmark: add_registrar
	/// 
	/// This benchmark tests adding registrars to the system
	/// Linear complexity based on the number of existing registrars
	#[benchmark]
	fn add_registrar(
		r: Linear<0, { T::MaxRegistrars::get() - 1 }>,
	) {
		// Pre-condition: add existing registrars for complexity
		for i in 0..r {
			let existing_registrar: T::AccountId = account("existing", i, 0);
			let _ = Identity::<T>::add_registrar(
				RawOrigin::Root.into(),
				existing_registrar
			);
		}
		
		let new_registrar: T::AccountId = account("new_registrar", r, 0);

		#[extrinsic_call]
		add_registrar(RawOrigin::Root, new_registrar.clone());

		// Verify registrar was added
		let registrars = Registrars::<T>::get();
		assert_eq!(registrars.len(), (r + 1) as usize);
		assert_eq!(
			registrars.last().unwrap().as_ref().unwrap().account,
			new_registrar
		);
	}

	impl_benchmark_test_suite!(Identity, crate::mock::new_test_ext(), crate::mock::Test);
}