use cosmwasm_std::{QuerierWrapper, QueryRequest, StdResult};

use ultra_base::oracle::{ExchangeRateResponse, UltraQuery, OracleQuery};

/// This is a helper wrapper to easily use our custom queries
pub struct UltraQuerier<'a> {
    querier: &'a QuerierWrapper<'a, UltraQuery>,
}

impl<'a> UltraQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper<'a, UltraQuery>) -> Self {
        UltraQuerier { querier }
    }

    pub fn query_exchange_rate<T: Into<String>>(
        &self,
        denom: T,
    ) -> StdResult<ExchangeRateResponse> {
        let query = UltraQuery::Oracle(OracleQuery::ExchangeRate {
            denom: denom.into(),
        });
        let request: QueryRequest<UltraQuery> = UltraQuery::into(query);
        self.querier.query(&request)
    }

}