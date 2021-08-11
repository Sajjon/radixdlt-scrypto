use crate::kernel::*;
use crate::types::*;

use sbor::{Decode, Describe, Encode};

/// A bucket that holds token resource.
#[derive(Debug, Describe, Encode, Decode)]
pub struct Tokens {
    bid: BID,
}

impl From<BID> for Tokens {
    fn from(bid: BID) -> Self {
        Self { bid }
    }
}

impl Into<BID> for Tokens {
    fn into(self) -> BID {
        self.bid
    }
}

impl Tokens {
    pub fn put(&mut self, other: Self) {
        let input = CombineBucketsInput {
            bucket: self.bid,
            other: other.bid,
        };
        let _: CombineBucketsOutput = call_kernel(COMBINE_BUCKETS, input);
    }

    pub fn take(&mut self, amount: U256) -> Self {
        let input = SplitBucketInput {
            bucket: self.bid,
            amount,
        };
        let output: SplitBucketOutput = call_kernel(SPLIT_BUCKET, input);

        output.bucket.into()
    }

    pub fn amount(&self) -> U256 {
        let input = GetBucketAmountInput { bucket: self.bid };
        let output: GetBucketAmountOutput = call_kernel(GET_BUCKET_AMOUNT, input);

        output.amount
    }

    pub fn resource(&self) -> Address {
        let input = GetBucketResourceInput { bucket: self.bid };
        let output: GetBucketResourceOutput = call_kernel(GET_BUCKET_RESOURCE, input);

        output.resource
    }
}