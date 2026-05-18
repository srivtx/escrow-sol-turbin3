use {
    anchor_lang::{
        InstructionData,
        solana_program::instruction::{AccountMeta, Instruction},
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    escrow::{ESCROW_SEED},
};

fn send_tx(svm: &mut LiteSVM, payer: &Keypair, instructions: Vec<Instruction>) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&instructions, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).unwrap();
}

fn send_tx_result(svm: &mut LiteSVM, payer: &Keypair, instructions: Vec<Instruction>) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&instructions, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx)
}

fn find_escrow_pdas(program_id: Pubkey, seed: u64, maker: Pubkey) -> (Pubkey, u8, Pubkey, u8) {
    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(
        &[ESCROW_SEED.as_bytes(), &seed.to_le_bytes(), maker.as_ref()],
        &program_id,
    );
    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[ESCROW_SEED.as_bytes(), b"vault", &seed.to_le_bytes(), maker.as_ref()],
        &program_id,
    );
    (escrow_pda, escrow_bump, vault_pda, vault_bump)
}

fn build_make_ix(
    program_id: Pubkey,
    maker: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    escrow_pda: Pubkey,
    vault_pda: Pubkey,
    seed: u64,
    deposit_amount: u64,
    receive_amount: u64,
) -> Instruction {
    let data = escrow::instruction::Make { seed, deposit_amount, receive_amount }.data();
    Instruction::new_with_bytes(
        program_id,
        &data,
        vec![
            AccountMeta::new(maker, true),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
    )
}

fn build_refund_ix(
    program_id: Pubkey,
    maker: Pubkey,
    escrow_pda: Pubkey,
    vault_pda: Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Refund {}.data(),
        vec![
            AccountMeta::new(maker, true),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
    )
}

fn build_take_ix(
    program_id: Pubkey,
    taker: Pubkey,
    maker: Pubkey,
    escrow_pda: Pubkey,
    vault_pda: Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Take {}.data(),
        vec![
            AccountMeta::new(taker, true),
            AccountMeta::new(maker, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
    )
}

#[test]
fn test_make() {
    let program_id = escrow::id();
    let maker = Keypair::new();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let seed = 42u64;

    let (escrow_pda, _, vault_pda, _) = find_escrow_pdas(program_id, seed, maker.pubkey());

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 2_000_000_000).unwrap();

    let deposit_amount = 100_000_000u64;
    let receive_amount = 50_000_000u64;

    let make_ix = build_make_ix(
        program_id, maker.pubkey(), mint_a, mint_b,
        escrow_pda, vault_pda, seed, deposit_amount, receive_amount,
    );
    send_tx(&mut svm, &maker, vec![make_ix]);

    let vault = svm.get_account(&vault_pda).unwrap();
    assert!(vault.lamports >= deposit_amount);

    let escrow = svm.get_account(&escrow_pda).unwrap();
    assert_eq!(escrow.owner, program_id);
}

#[test]
fn test_refund() {
    let program_id = escrow::id();
    let maker = Keypair::new();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let seed = 42u64;

    let (escrow_pda, _, vault_pda, _) = find_escrow_pdas(program_id, seed, maker.pubkey());

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 2_000_000_000).unwrap();

    let deposit_amount = 100_000_000u64;
    let receive_amount = 50_000_000u64;

    let make_ix = build_make_ix(
        program_id, maker.pubkey(), mint_a, mint_b,
        escrow_pda, vault_pda, seed, deposit_amount, receive_amount,
    );
    send_tx(&mut svm, &maker, vec![make_ix]);

    let maker_before = svm.get_account(&maker.pubkey()).unwrap().lamports;

    let refund_ix = build_refund_ix(program_id, maker.pubkey(), escrow_pda, vault_pda);
    send_tx(&mut svm, &maker, vec![refund_ix]);

    let maker_after = svm.get_account(&maker.pubkey()).unwrap().lamports;
    // Maker should have gotten the deposit back (minus some rent, but close sends rent back too)
    assert!(maker_after > maker_before);

    // Escrow should be closed
    assert!(svm.get_account(&escrow_pda).is_none());
}

#[test]
fn test_take() {
    let program_id = escrow::id();
    let maker = Keypair::new();
    let taker = Keypair::new();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let seed = 42u64;

    let (escrow_pda, _, vault_pda, _) = find_escrow_pdas(program_id, seed, maker.pubkey());

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 2_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 2_000_000_000).unwrap();

    let deposit_amount = 100_000_000u64;
    let receive_amount = 50_000_000u64;

    let make_ix = build_make_ix(
        program_id, maker.pubkey(), mint_a, mint_b,
        escrow_pda, vault_pda, seed, deposit_amount, receive_amount,
    );
    send_tx(&mut svm, &maker, vec![make_ix]);

    let taker_before = svm.get_account(&taker.pubkey()).unwrap().lamports;
    let maker_before = svm.get_account(&maker.pubkey()).unwrap().lamports;

    let take_ix = build_take_ix(program_id, taker.pubkey(), maker.pubkey(), escrow_pda, vault_pda);
    send_tx(&mut svm, &taker, vec![take_ix]);

    let taker_after = svm.get_account(&taker.pubkey()).unwrap().lamports;
    let maker_after = svm.get_account(&maker.pubkey()).unwrap().lamports;

    // Taker should have received the deposit but paid the receive_amount
    let taker_delta = taker_after as i64 - taker_before as i64;
    assert!(taker_delta > (deposit_amount as i64 - receive_amount as i64 - 10_000_000));
    assert!(taker_delta < (deposit_amount as i64 - receive_amount as i64 + 10_000_000));

    // Maker should have received the receive_amount
    let maker_delta = maker_after as i64 - maker_before as i64;
    assert!(maker_delta > (receive_amount as i64 - 10_000_000));
    assert!(maker_delta < (receive_amount as i64 + 10_000_000));

    // Escrow should be closed
    assert!(svm.get_account(&escrow_pda).is_none());
}

#[test]
fn test_unauthorized_refund_fails() {
    let program_id = escrow::id();
    let maker = Keypair::new();
    let attacker = Keypair::new();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let seed = 42u64;

    let (escrow_pda, _, vault_pda, _) = find_escrow_pdas(program_id, seed, maker.pubkey());

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 2_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let make_ix = build_make_ix(
        program_id, maker.pubkey(), mint_a, mint_b,
        escrow_pda, vault_pda, seed, 100_000_000, 50_000_000,
    );
    send_tx(&mut svm, &maker, vec![make_ix]);

    let refund_ix = build_refund_ix(program_id, attacker.pubkey(), escrow_pda, vault_pda);
    assert!(send_tx_result(&mut svm, &attacker, vec![refund_ix]).is_err());
}

#[test]
fn test_full_flow() {
    let program_id = escrow::id();
    let maker = Keypair::new();
    let taker = Keypair::new();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let seed = 42u64;

    let (escrow_pda, _, vault_pda, _) = find_escrow_pdas(program_id, seed, maker.pubkey());

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 2_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 2_000_000_000).unwrap();

    let deposit_amount = 100_000_000u64;
    let receive_amount = 50_000_000u64;

    // MAKE
    let make_ix = build_make_ix(
        program_id, maker.pubkey(), mint_a, mint_b,
        escrow_pda, vault_pda, seed, deposit_amount, receive_amount,
    );
    send_tx(&mut svm, &maker, vec![make_ix]);

    let vault = svm.get_account(&vault_pda).unwrap();
    assert!(vault.lamports >= deposit_amount);

    // TAKE
    let taker_before = svm.get_account(&taker.pubkey()).unwrap().lamports;
    let maker_before = svm.get_account(&maker.pubkey()).unwrap().lamports;

    let take_ix = build_take_ix(program_id, taker.pubkey(), maker.pubkey(), escrow_pda, vault_pda);
    send_tx(&mut svm, &taker, vec![take_ix]);

    let taker_after = svm.get_account(&taker.pubkey()).unwrap().lamports;
    let maker_after = svm.get_account(&maker.pubkey()).unwrap().lamports;

    let taker_delta = taker_after as i64 - taker_before as i64;
    assert!(taker_delta > (deposit_amount as i64 - receive_amount as i64 - 10_000_000));
    assert!(taker_delta < (deposit_amount as i64 - receive_amount as i64 + 10_000_000));

    let maker_delta = maker_after as i64 - maker_before as i64;
    assert!(maker_delta > (receive_amount as i64 - 10_000_000));
    assert!(maker_delta < (receive_amount as i64 + 10_000_000));

    assert!(svm.get_account(&escrow_pda).is_none());
}
