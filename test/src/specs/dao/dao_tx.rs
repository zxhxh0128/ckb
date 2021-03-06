use crate::specs::dao::dao_user::DAOUser;
use crate::specs::dao::dao_verifier::DAOVerifier;
use crate::specs::dao::utils::{ensure_committed, goto_target_point};
use crate::utils::{assert_send_transaction_fail, generate_utxo_set};
use crate::{Net, Spec};
use ckb_chain_spec::ChainSpec;
use ckb_types::core::EpochNumberWithFraction;
use ckb_types::{core::Capacity, prelude::*};

pub struct WithdrawDAO;

impl Spec for WithdrawDAO {
    crate::name!("withdraw_dao");

    fn modify_chain_spec(&self) -> Box<dyn Fn(&mut ChainSpec) -> ()> {
        Box::new(|spec_config| {
            spec_config.params.genesis_epoch_length = 2;
            spec_config.params.epoch_duration_target = 2;
            spec_config.params.permanent_difficulty_in_dummy = true;
        })
    }

    fn run(&self, net: &mut Net) {
        let node = &net.nodes[0];
        let utxos = generate_utxo_set(node, 21);
        let mut user = DAOUser::new(node, utxos);

        ensure_committed(node, &user.deposit());
        node.generate_blocks(20); // Time makes interest
        ensure_committed(node, &user.prepare());

        let withdrawal = user.withdraw();
        let since = EpochNumberWithFraction::from_full_value(
            withdrawal.inputs().get(0).unwrap().since().unpack(),
        );
        goto_target_point(node, since);
        ensure_committed(node, &withdrawal);
        DAOVerifier::init(node).verify();
    }
}

pub struct WithdrawDAOWithOverflowCapacity;

impl Spec for WithdrawDAOWithOverflowCapacity {
    crate::name!("withdraw_dao_with_overflow_capacity");

    fn modify_chain_spec(&self) -> Box<dyn Fn(&mut ChainSpec) -> ()> {
        Box::new(|spec_config| {
            spec_config.params.genesis_epoch_length = 2;
            spec_config.params.epoch_duration_target = 2;
            spec_config.params.permanent_difficulty_in_dummy = true;
        })
    }

    fn run(&self, net: &mut Net) {
        let node = &net.nodes[0];
        let utxos = generate_utxo_set(node, 21);
        let mut user = DAOUser::new(node, utxos);

        ensure_committed(node, &user.deposit());
        node.generate_blocks(20); // Time makes interest
        ensure_committed(node, &user.prepare());

        let withdrawal = user.withdraw();
        let invalid_withdrawal = {
            let outputs: Vec<_> = withdrawal
                .outputs()
                .into_iter()
                .map(|cell_output| {
                    let old_capacity: Capacity = cell_output.capacity().unpack();
                    let new_capacity = old_capacity.safe_add(Capacity::one()).unwrap();
                    cell_output
                        .as_builder()
                        .capacity(new_capacity.pack())
                        .build()
                })
                .collect();
            withdrawal
                .as_advanced_builder()
                .set_outputs(outputs)
                .build()
        };
        let since = EpochNumberWithFraction::from_full_value(
            withdrawal.inputs().get(0).unwrap().since().unpack(),
        );
        goto_target_point(node, since);
        assert_send_transaction_fail(node, &invalid_withdrawal, "CapacityOverflow");
        ensure_committed(node, &withdrawal);
    }
}
