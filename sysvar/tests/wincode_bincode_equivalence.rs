#![allow(deprecated)]

use {
    proptest::prelude::*,
    serde::{de::DeserializeOwned, Serialize},
    solana_fee_calculator::FeeCalculator,
    solana_hash::Hash,
    solana_sysvar::{
        clock::{self, Clock},
        epoch_rewards::{self, EpochRewards},
        epoch_schedule::{self, EpochSchedule},
        fees::{self, Fees},
        last_restart_slot::{self, LastRestartSlot},
        recent_blockhashes::{self, IterItem, RecentBlockhashes},
        rent::{self, Rent},
        rewards::{self, Rewards},
        slot_hashes::{self, SlotHashes},
        slot_history::{self, SlotHistory},
        stake_history::{self, StakeHistory, StakeHistoryEntry},
    },
};

fn assert_equivalent<T>(value: &T, account_size: usize)
where
    T: Serialize
        + DeserializeOwned
        + wincode::Serialize<Src = T>
        + wincode::DeserializeOwned<Dst = T>,
{
    let bincode_bytes = bincode::serialize(value).unwrap();
    let wincode_bytes = wincode::serialize(value).unwrap();

    assert_eq!(bincode_bytes, wincode_bytes);
    assert_eq!(
        bincode::serialized_size(value).unwrap(),
        bincode_bytes.len() as u64
    );
    assert_eq!(
        wincode::serialized_size(value).unwrap(),
        wincode_bytes.len() as u64
    );
    let bincode_decoded = bincode::deserialize::<T>(&wincode_bytes).unwrap();
    let wincode_decoded = wincode::deserialize::<T>(&bincode_bytes).unwrap();
    assert_eq!(wincode::serialize(&bincode_decoded).unwrap(), wincode_bytes);
    assert_eq!(bincode::serialize(&wincode_decoded).unwrap(), bincode_bytes);

    // Sysvar accounts are generally larger than the current encoded value.
    // Verify both writers preserve the trailing account data and both readers
    // accept it.
    assert!(bincode_bytes.len() <= account_size);
    let mut expected_account = vec![0xA5; account_size];
    expected_account[..bincode_bytes.len()].copy_from_slice(&bincode_bytes);
    let mut bincode_account = vec![0xA5; account_size];
    let mut wincode_account = vec![0xA5; account_size];
    bincode::serialize_into(bincode_account.as_mut_slice(), value).unwrap();
    wincode::serialize_into(wincode_account.as_mut_slice(), value).unwrap();
    assert_eq!(bincode_account, expected_account);
    assert_eq!(wincode_account, expected_account);
    assert_eq!(
        wincode::serialize(&bincode::deserialize::<T>(&wincode_account).unwrap()).unwrap(),
        wincode_bytes
    );
    assert_eq!(
        bincode::serialize(&wincode::deserialize::<T>(&bincode_account).unwrap()).unwrap(),
        bincode_bytes
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn clock_is_bincode_equivalent(
        slot in any::<u64>(),
        epoch_start_timestamp in any::<i64>(),
        epoch in any::<u64>(),
        leader_schedule_epoch in any::<u64>(),
        unix_timestamp in any::<i64>(),
    ) {
        assert_equivalent(
            &Clock {
                slot,
                epoch_start_timestamp,
                epoch,
                leader_schedule_epoch,
                unix_timestamp,
            },
            clock::SIZE,
        );
    }

    #[test]
    fn epoch_rewards_is_bincode_equivalent(
        distribution_starting_block_height in any::<u64>(),
        num_partitions in any::<u64>(),
        parent_blockhash in any::<[u8; 32]>(),
        total_points in any::<u128>(),
        total_rewards in any::<u64>(),
        distributed_rewards in any::<u64>(),
        active in any::<bool>(),
    ) {
        assert_equivalent(
            &EpochRewards {
                distribution_starting_block_height,
                num_partitions,
                parent_blockhash: Hash::new_from_array(parent_blockhash),
                total_points,
                total_rewards,
                distributed_rewards,
                active,
            },
            epoch_rewards::SIZE,
        );
    }

    #[test]
    fn epoch_schedule_is_bincode_equivalent(
        slots_per_epoch in any::<u64>(),
        leader_schedule_slot_offset in any::<u64>(),
        warmup in any::<bool>(),
        first_normal_epoch in any::<u64>(),
        first_normal_slot in any::<u64>(),
    ) {
        assert_equivalent(
            &EpochSchedule {
                slots_per_epoch,
                leader_schedule_slot_offset,
                warmup,
                first_normal_epoch,
                first_normal_slot,
            },
            epoch_schedule::SIZE,
        );
    }

    #[test]
    fn fees_is_bincode_equivalent(lamports_per_signature in any::<u64>()) {
        assert_equivalent(
            &Fees::new(&FeeCalculator::new(lamports_per_signature)),
            fees::SIZE,
        );
    }

    #[test]
    fn last_restart_slot_is_bincode_equivalent(slot in any::<u64>()) {
        assert_equivalent(
            &LastRestartSlot { last_restart_slot: slot },
            last_restart_slot::SIZE,
        );
    }

    #[test]
    fn rent_is_bincode_equivalent(
        lamports_per_byte in any::<u64>(),
        exemption_threshold in any::<[u8; 8]>(),
        burn_percent in any::<u8>(),
    ) {
        assert_equivalent(
            &Rent {
                lamports_per_byte,
                exemption_threshold,
                burn_percent,
            },
            rent::SIZE,
        );
    }

    #[test]
    fn rewards_is_bincode_equivalent(
        validator_point_value in any::<u64>().prop_map(f64::from_bits),
        unused in any::<u64>().prop_map(f64::from_bits),
    ) {
        assert_equivalent(
            &Rewards {
                validator_point_value,
                unused,
            },
            rewards::SIZE,
        );
    }

    #[test]
    fn recent_blockhashes_is_bincode_equivalent(
        entries in prop_oneof![
            prop::collection::vec(
                (any::<[u8; 32]>(), any::<u64>()),
                0..=recent_blockhashes::MAX_ENTRIES,
            ),
            prop::collection::vec(
                (any::<[u8; 32]>(), any::<u64>()),
                recent_blockhashes::MAX_ENTRIES..=recent_blockhashes::MAX_ENTRIES,
            ),
        ],
    ) {
        let hashes: Vec<_> = entries
            .iter()
            .map(|(hash, _)| Hash::new_from_array(*hash))
            .collect();
        let recent_blockhashes: RecentBlockhashes = entries
            .iter()
            .zip(&hashes)
            .enumerate()
            .map(|(slot, ((_, lamports_per_signature), hash))| {
                IterItem(slot as u64, hash, *lamports_per_signature)
            })
            .collect();
        assert_equivalent(&recent_blockhashes, recent_blockhashes::SIZE);
    }

    #[test]
    fn slot_hashes_is_bincode_equivalent(
        hashes in prop_oneof![
            prop::collection::vec(any::<[u8; 32]>(), 0..=solana_slot_hashes::MAX_ENTRIES),
            prop::collection::vec(
                any::<[u8; 32]>(),
                solana_slot_hashes::MAX_ENTRIES..=solana_slot_hashes::MAX_ENTRIES,
            ),
        ],
    ) {
        let mut slot_hashes = SlotHashes::default();
        for (slot, hash) in hashes.into_iter().enumerate() {
            slot_hashes.add(slot as u64, Hash::new_from_array(hash));
        }
        assert_equivalent(&slot_hashes, slot_hashes::SIZE);
    }

    #[test]
    fn stake_history_is_bincode_equivalent(
        entries in prop_oneof![
            prop::collection::vec(
                (any::<u64>(), any::<u64>(), any::<u64>()),
                0..=stake_history::MAX_ENTRIES,
            ),
            prop::collection::vec(
                (any::<u64>(), any::<u64>(), any::<u64>()),
                stake_history::MAX_ENTRIES..=stake_history::MAX_ENTRIES,
            ),
        ],
    ) {
        let mut stake_history = StakeHistory::default();
        for (epoch, (effective, activating, deactivating)) in entries.into_iter().enumerate() {
            stake_history.add(
                epoch as u64,
                StakeHistoryEntry {
                    effective,
                    activating,
                    deactivating,
                },
            );
        }
        assert_equivalent(&stake_history, stake_history::SIZE);
    }

    #[test]
    fn slot_history_is_bincode_equivalent(
        base in 0..=u64::MAX - solana_slot_history::MAX_ENTRIES,
        offsets in prop::collection::btree_set(
            0..solana_slot_history::MAX_ENTRIES,
            0..=2_048,
        ),
    ) {
        let mut slot_history = SlotHistory::default();
        for offset in offsets {
            slot_history.add(base + offset);
        }
        assert_equivalent(&slot_history, slot_history::SIZE);
    }

    #[test]
    fn invalid_epoch_schedule_boolean_is_rejected_equivalently(invalid_bool in 2..=u8::MAX) {
        let mut bytes = bincode::serialize(&EpochSchedule::default()).unwrap();
        bytes[16] = invalid_bool;
        assert!(bincode::deserialize::<EpochSchedule>(&bytes).is_err());
        assert!(wincode::deserialize::<EpochSchedule>(&bytes).is_err());
    }

    #[test]
    fn invalid_epoch_rewards_boolean_is_rejected_equivalently(invalid_bool in 2..=u8::MAX) {
        let mut bytes = bincode::serialize(&EpochRewards::default()).unwrap();
        let active_offset = bytes.len() - 1;
        bytes[active_offset] = invalid_bool;
        assert!(bincode::deserialize::<EpochRewards>(&bytes).is_err());
        assert!(wincode::deserialize::<EpochRewards>(&bytes).is_err());
    }
}

// Related: https://github.com/anza-xyz/wincode/blob/c002a5e2631b586e0199aa7417ba7c9b71bd2bc9/wincode/src/schema/external/bv.rs#L270-L300
#[test]
fn slot_history_overallocated_bitvec_encoding_diverges() {
    let mut slot_history = SlotHistory::default();
    slot_history.bits.truncate(10);

    let bincode_bytes = bincode::serialize(&slot_history).unwrap();
    let wincode_bytes = wincode::serialize(&slot_history).unwrap();

    // bincode serializes the full backing allocation, while wincode serializes
    // only the blocks required by the logical bit length.
    assert_ne!(bincode_bytes, wincode_bytes);
    assert!(bincode_bytes.len() > wincode_bytes.len());

    // Despite the different encodings, both codecs decode both representations
    // to the same logical SlotHistory value.
    let bincode_from_bincode = bincode::deserialize::<SlotHistory>(&bincode_bytes).unwrap();
    let bincode_from_wincode = bincode::deserialize::<SlotHistory>(&wincode_bytes).unwrap();
    let wincode_from_wincode = wincode::deserialize::<SlotHistory>(&wincode_bytes).unwrap();
    let wincode_from_bincode = wincode::deserialize::<SlotHistory>(&bincode_bytes).unwrap();

    assert_eq!(bincode_from_bincode, slot_history);
    assert_eq!(bincode_from_wincode, slot_history);
    assert_eq!(wincode_from_wincode, slot_history);
    assert_eq!(wincode_from_bincode, slot_history);
}
