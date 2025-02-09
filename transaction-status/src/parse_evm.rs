use crate::parse_instruction::{
    check_num_accounts, ParsableProgram, ParseInstructionError, ParsedInstructionEnum,
};
use bincode::deserialize;
use evm_rpc::RPCTransaction;
use serde_json::json;
use solana_evm_loader_program::instructions::{EvmBigTransaction, EvmInstruction};
use solana_sdk::{instruction::CompiledInstruction, pubkey::Pubkey};

pub fn parse_evm(
    instruction: &CompiledInstruction,
    account_keys: &[Pubkey],
) -> Result<ParsedInstructionEnum, ParseInstructionError> {
    let evm_instruction: EvmInstruction = deserialize(&instruction.data)
        .map_err(|_| ParseInstructionError::InstructionNotParsable(ParsableProgram::Stake))?;
    match instruction.accounts.iter().max() {
        Some(index) if (*index as usize) < account_keys.len() => {}
        _ => {
            // Runtime should prevent this from ever happening
            return Err(ParseInstructionError::InstructionKeyMismatch(
                ParsableProgram::Evm,
            ));
        }
    }

    match evm_instruction {
        EvmInstruction::EvmTransaction { evm_tx } => {
            let info = if instruction.accounts.len() >= 2 {
                json!({
                    "bridgeAccount":  account_keys[instruction.accounts[1] as usize].to_string(),
                    "transaction": RPCTransaction::from_transaction(evm_tx.into()).map_err(|_|ParseInstructionError::InstructionKeyMismatch(
                        ParsableProgram::Evm,
                    ))?,
                })
            } else {
                json!({
                    "transaction": RPCTransaction::from_transaction(evm_tx.into()).map_err(|_|ParseInstructionError::InstructionKeyMismatch(
                        ParsableProgram::Evm,
                    ))?,
                })
            };

            Ok(ParsedInstructionEnum {
                instruction_type: "evmTransaction".to_string(),
                info,
            })
        }
        EvmInstruction::EvmAuthorizedTransaction { from, unsigned_tx } => {
            check_num_stake_accounts(&instruction.accounts, 2)?;
            let tx = evm_state::UnsignedTransactionWithCaller {
                caller: from,
                unsigned_tx,
                signed_compatible: true,
                chain_id: 0,
            };
            let info = json!({
                "programAccount":  account_keys[instruction.accounts[1] as usize].to_string(),
                "transaction": RPCTransaction::from_transaction(tx.into()).map_err(|_|ParseInstructionError::InstructionKeyMismatch(
                    ParsableProgram::Evm,
                ))?,
            });

            Ok(ParsedInstructionEnum {
                instruction_type: "evmAuthorizedTransaction".to_string(),
                info,
            })
        }

        EvmInstruction::FreeOwnership {} => Ok(ParsedInstructionEnum {
            instruction_type: "freeOwnership".to_string(),
            info: Default::default(),
        }),
        EvmInstruction::SwapNativeToEther {
            lamports,
            evm_address,
        } => {
            check_num_stake_accounts(&instruction.accounts, 2)?;
            let info = json!({
                "fromNativeAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                "toEvmAccount": evm_address.to_string(),
                "lamports": lamports,
            });
            Ok(ParsedInstructionEnum {
                instruction_type: "swapNativeToEvm".to_string(),
                info,
            })
        }
        EvmInstruction::EvmBigTransaction(big_tx) => match big_tx {
            EvmBigTransaction::EvmTransactionAllocate { size } => {
                check_num_stake_accounts(&instruction.accounts, 2)?;
                let info = json!({
                    "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                    "size": size,
                });
                Ok(ParsedInstructionEnum {
                    instruction_type: "evmBigTransactionAllocate".to_string(),
                    info,
                })
            }

            EvmBigTransaction::EvmTransactionWrite { offset, data } => {
                check_num_stake_accounts(&instruction.accounts, 2)?;
                let info = json!({
                    "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                    "offset": offset,
                    "data": base64::encode(data)
                });
                Ok(ParsedInstructionEnum {
                    instruction_type: "evmBigTransactionWrite".to_string(),
                    info,
                })
            }

            EvmBigTransaction::EvmTransactionExecute {} => {
                check_num_stake_accounts(&instruction.accounts, 2)?;
                let info = if instruction.accounts.len() >= 3 {
                    json!({
                        "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                        "bridgeAccount":  account_keys[instruction.accounts[2] as usize].to_string(),
                    })
                } else {
                    json!({
                        "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                    })
                };

                Ok(ParsedInstructionEnum {
                    instruction_type: "evmBigTransactionExecute".to_string(),
                    info,
                })
            }
            EvmBigTransaction::EvmTransactionExecuteUnsigned { from } => {
                check_num_stake_accounts(&instruction.accounts, 2)?;
                let info = if instruction.accounts.len() >= 3 {
                    json!({
                        "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                        "bridgeAccount":  account_keys[instruction.accounts[2] as usize].to_string(),
                        "from": from.to_string(),
                    })
                } else {
                    json!({
                        "storageAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                        "from": from.to_string(),
                    })
                };

                Ok(ParsedInstructionEnum {
                    instruction_type: "evmBigTransactionExecute".to_string(),
                    info,
                })
            }
        },
    }
}

fn check_num_stake_accounts(accounts: &[u8], num: usize) -> Result<(), ParseInstructionError> {
    check_num_accounts(accounts, num, ParsableProgram::Evm)
}
