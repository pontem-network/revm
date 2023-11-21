use crate::{
    db::Database,
    handler::Handler,
    inspector_instruction,
    interpreter::{
        gas::initial_tx_gas,
        opcode::{make_boxed_instruction_table, make_instruction_table, InstructionTables},
        CallContext, CallInputs, CallScheme, CreateInputs, Host, Interpreter, InterpreterAction,
        InterpreterResult, SelfDestructResult, SharedMemory, Transfer,
    },
    journaled_state::JournaledState,
    precompile::Precompiles,
    primitives::{
        specification, Address, Bytecode, Bytes, EVMError, EVMResult, Env, InvalidTransaction, Log,
        Output, Spec, SpecId::*, TransactTo, B256, U256,
    },
    CallStackFrame, Context, EvmContext, FrameOrResult, Inspector,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use auto_impl::auto_impl;
use core::{fmt, marker::PhantomData, ops::Range};

/// Register external handles.
pub trait RegisterHandler<DB: Database> {
    /// Register external handler.
    fn register_handler<'a, SPEC: Spec>(
        &self,
        handler: Handler<'a, Self, DB>,
    ) -> Handler<'a, Self, DB>
    where
        Self: Sized+'a,
        DB: 'a,
    {
        handler
    }
}

/// Default registered handler that produces default mainnet handler.
#[derive(Default)]
pub struct MainnetHandle {}

impl<DB: Database> RegisterHandler<DB> for MainnetHandle {}

pub struct InspectorHandle<'a, DB: Database, INS: Inspector<DB>> {
    pub inspector: &'a mut INS,
    pub _phantomdata: PhantomData<DB>,
}

impl<'a, DB: Database, INS: Inspector<DB>> RegisterHandler<DB> for InspectorHandle<'a, DB, INS> {
    fn register_handler<'b, SPEC: Spec>(
        &self,
        mut handler: Handler<'b, Self, DB>,
    ) -> Handler<'b, Self, DB>
    where
        Self: Sized+'b,
        DB: 'b,
    {

        

        // return frame handle
        let old_handle = handler.frame_return.clone();
        handler.frame_return = Arc::new(
            move |context, mut child, parent, memory, mut result| -> Option<InterpreterResult> {
                let inspector = &mut context.external.inspector;
                result = if child.is_create {
                    let (result, address) =
                        inspector.create_end(&mut context.evm, result, child.created_address);
                    child.created_address = address;
                    result
                } else {
                    inspector.call_end(&mut context.evm, result)
                };
                let output = old_handle(context, child, parent, memory, result);
                output
            },
        );

        handler
    }
}

pub struct ExternalData<DB: Database> {
    pub flagg: bool,
    pub phantom: PhantomData<DB>,
}

impl<DB: Database> RegisterHandler<DB> for ExternalData<DB> {
    fn register_handler<'a, SPEC: Spec>(
        &self,
        mut handler: Handler<'a, Self, DB>,
    ) -> Handler<'a, Self, DB>
    where
        DB: 'a,
    {
        let old_handle = handler.reimburse_caller.clone();
        handler.reimburse_caller = Arc::new(move |data, gas| old_handle(data, gas));
        handler
    }
}
