use alloy::{
    network::AnyNetwork,
    primitives::{Address, U256},
    providers::Provider,
    sol,
    sol_types::{SolCall, SolConstructor, SolEvent},
    transports::Transport,
};
use std::sync::Arc;

use crate::{
    amm::{AutomatedMarketMaker, AMM},
    errors::AMMError,
};

use super::UniswapV2Pool;

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    IGetUniswapV2PairsBatchRequest,
    "src/amm/uniswap_v2/batch_request/GetUniswapV2PairsBatchRequestABI.json"
}

sol! {
    contract IGetUniswapV2PairsBatchReturn {
        function constructorReturn() external view returns (address[] memory);
    }
}

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    IGetUniswapV2PoolDataBatchRequest,
    "src/amm/uniswap_v2/batch_request/GetUniswapV2PoolDataBatchRequestABI.json"
}

sol! {
    contract IGetUniswapV2PoolDataBatchReturn {
        function constructorReturn() external view returns ((address, uint8, address, uint8, uint112, uint112)[] memory);
    }
}

pub async fn get_pairs_batch_request<T: Transport + Clone, P: Provider<T, AnyNetwork>>(
    factory: Address,
    from: U256,
    step: U256,
    provider: Arc<P>,
) -> Result<Vec<Address>, AMMError<T>> {
    let deployer = IGetUniswapV2PairsBatchRequest::deploy_builder(provider, from, step, factory)
        .with_sol_decoder::<IGetUniswapV2PairsBatchReturn::constructorReturnCall>();

    let IGetUniswapV2PairsBatchReturn::constructorReturnReturn { _0: pairs } =
        deployer.call().await?;

    Ok(pairs)
}

pub async fn get_amm_data_batch_request<T: Transport + Clone, P: Provider<T, AnyNetwork>>(
    amms: &mut [AMM],
    provider: Arc<P>,
) -> Result<(), AMMError<T>> {
    let mut target_addresses = vec![];
    for amm in amms.iter() {
        target_addresses.push(amm.address());
    }

    let deployer =
        IGetUniswapV2PoolDataBatchRequest::deploy_builder(provider.clone(), target_addresses)
            .with_sol_decoder::<IGetUniswapV2PoolDataBatchReturn::constructorReturnCall>();
    let IGetUniswapV2PoolDataBatchReturn::constructorReturnReturn { _0: amms_data } =
        deployer.call().await?;

    let mut pool_idx = 0;

    for amm_data in amms_data {
        if !amm_data.0.is_zero() {
            if let AMM::UniswapV2Pool(uniswap_v2_pool) = amms
                .get_mut(pool_idx)
                .expect("Pool idx should be in bounds")
            {
                uniswap_v2_pool.token_a = amm_data.0;
                uniswap_v2_pool.token_a_decimals = amm_data.1;
                uniswap_v2_pool.token_b = amm_data.2;
                uniswap_v2_pool.token_b_decimals = amm_data.3;
                uniswap_v2_pool.reserve_0 = amm_data.4;
                uniswap_v2_pool.reserve_1 = amm_data.5;

                tracing::trace!(?uniswap_v2_pool);
            }

            pool_idx += 1;
        }
    }

    Ok(())
}

pub async fn get_v2_pool_data_batch_request<T: Transport + Clone, P: Provider<T, AnyNetwork>>(
    pool: &mut UniswapV2Pool,
    provider: Arc<P>,
) -> Result<(), AMMError<T>> {
    let deployer =
        IGetUniswapV2PoolDataBatchRequest::deploy_builder(provider.clone(), vec![pool.address])
            .with_sol_decoder::<IGetUniswapV2PoolDataBatchReturn::constructorReturnCall>();
    let IGetUniswapV2PoolDataBatchReturn::constructorReturnReturn { _0: pools_data } =
        deployer.call().await?;

    // make sure returned pool data is 1
    let pools_data_len = pools_data.len();
    if pools_data_len != 1_usize {
        return Err(AMMError::EyreError(eyre::eyre!(
            "Unexpected return length, expected 1, returned {pools_data_len}"
        )));
    }

    if !pools_data[0].0.is_zero() {
        pool.token_a = pools_data[0].0;
        pool.token_a_decimals = pools_data[0].1;
        pool.token_b = pools_data[0].2;
        pool.token_b_decimals = pools_data[0].3;
        pool.reserve_0 = pools_data[0].4;
        pool.reserve_1 = pools_data[0].5;

        tracing::trace!(?pool);
    }

    Ok(())
}
