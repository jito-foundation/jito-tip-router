use anchor_lang::AccountDeserialize;
use itertools::Itertools;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::{
    derive_claim_status_account_address, TipDistributionAccount, CLAIM_STATUS_SIZE, CONFIG_SEED,
};
use jito_tip_router_client::instructions::ClaimWithPayerBuilder;
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use legacy_meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTreeCollection as LegacyGeneratedMerkleTreeCollection;
use legacy_tip_router_operator_cli::claim::ClaimMevError as LegacyClaimMevError;
use log::{info, warn};
use meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTreeCollection;
use rand::{prelude::SliceRandom, thread_rng};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    fee_calculator::DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE,
    native_token::{lamports_to_sol, LAMPORTS_PER_SOL},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::sync::Arc;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use std::{path::PathBuf, str::FromStr};
use thiserror::Error;
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;

use crate::{
    account_analyzer::AccountAnalysisEntry,
    merkle_tree_collection_file_name, priority_fees,
    rpc_utils::{get_batched_accounts, send_until_blockhash_expires},
    Cli,
};

const JSON_808: &str = r#"
[
  {
    "vote_account": "KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag",
    "mev_revenue": "128250941158",
    "claim_status_account": "8YBTPmJ7c12od7KFJzhGuhktVxVkCJMcs99bZhRwM37i"
  },
  {
    "vote_account": "QhyTEHb5JkMBki8Lq1npsaixefUyMXWJtbxK6jNjxnn",
    "mev_revenue": "8330078573",
    "claim_status_account": "5uM33U31VsANx5CD15Dh1SVZrkctsApQrSZTd2QRzrV4"
  },
  {
    "vote_account": "StepeLdhJ2znRjHcZdjwMWsC4nTRURNKQY8Nca82LJp",
    "mev_revenue": "10468294325",
    "claim_status_account": "5dw88wDEe3cfUEUq1aexXdy7eeP5j6pJ6SU2i8tcSRfZ"
  },
  {
    "vote_account": "VoTEJDVw84uZvDWcMXYgfNAVcLETHsPyPEc2nTpwZPa",
    "mev_revenue": "587857644",
    "claim_status_account": "H4Eq4jgELj8Ys6Tk9QMarXXwcQc9VBt76sazZUwhY6kn"
  },
  {
    "vote_account": "VoteMYitKq7mruk9QPJRUgryYbSkyZKBuvnL1VTgoMq",
    "mev_revenue": "116348706",
    "claim_status_account": "7Pe4Kk9YsBLzkna7Lmdinx1gnPcR8cyHkJfqsAocp94S"
  },
  {
    "vote_account": "iZADA4YKVRJZJaDUV3j79DzyK4VJkK3DGTfvvqvbC1K",
    "mev_revenue": "121746521543",
    "claim_status_account": "FkvXJn2cxh6XP9rDPWPtHtaFQm6q3u4JF62qSQXEQVXZ"
  },
  {
    "vote_account": "irKsY8c3sQur1XaYuQ811hzsEQJ5Hq3Yu3AAoXYnp8W",
    "mev_revenue": "2170126571",
    "claim_status_account": "E6qhr2yD25TJCpCbE62HbnLoRAncc1qXae6BbiHfoK44"
  },
  {
    "vote_account": "oDDiLXv87uRfbAB8PZthCtQyqof2Jomv7fpTeoBp6AY",
    "mev_revenue": "1570501897",
    "claim_status_account": "5AiPM5ZK1VHbiCnUdacLdFURumw8CJs1qH15KoqN4gL3"
  },
  {
    "vote_account": "oixpqSNX7CKWHw93ViA8u1CcLzZXDmacKJjV4AvxMZE",
    "mev_revenue": "92021779981",
    "claim_status_account": "FHr4fvniww7kVFfF4ksgBFi9cKo34K3K1AgNfHvBFkSV"
  },
  {
    "vote_account": "privSGy4XbFCjzEkdLXssV8xMWRWWiWbJeDzh1emUyL",
    "mev_revenue": "44317114",
    "claim_status_account": "EGWHMzp4K8CnquN6RG3g9BS1efoVT7pvsEMG12Se81Aj"
  },
  {
    "vote_account": "reyYoUdFgtDLxAWW1hyn5xk2PHstA3j8zUrerQi9Ayq",
    "mev_revenue": "1443007378",
    "claim_status_account": "2LoqGbiowuy8K4GAN2AMxVkbfPHbyubGGuyofQJ8eQz1"
  },
  {
    "vote_account": "2PEyBgsPYBQ8pMdXQtEaPGNqWQHE9GCnmV2tTVN4GMru",
    "mev_revenue": "2741741537",
    "claim_status_account": "8VvwTVYPEtWY5YSHr3LYphxP78NMSkYCmqkBvDT68cap"
  },
  {
    "vote_account": "2qD6yvLwy3ckWxsS1iQwrkCjLgcvH1u9PLu5m9KRRn5x",
    "mev_revenue": "4917131428",
    "claim_status_account": "rnX5QKchUgDEnbAiHVDGJZbYGGyYbbvardDG6LDf8PA"
  },
  {
    "vote_account": "2uXzxR2EZVaHE3CuaDaUJ8C9Qr54LMfwkY5BtRPAzbPE",
    "mev_revenue": "109784027438",
    "claim_status_account": "BdzWD35CWYeaYYAfMtAmru6HBfgFCgnwdfKESJ3hPWqL"
  },
  {
    "vote_account": "33hurzEz6aEnzfESL6pnNyR6DCgcKzssT1pwSzDCBTRQ",
    "mev_revenue": "85770932452",
    "claim_status_account": "5h9eweDkbLn9f21oxH7GAmszqCQTQzr1T1sWCiJTx3sL"
  },
  {
    "vote_account": "37BPVW1Ne1XHrzK15xguAS2BTdobVfThDzTE2mv8SsnJ",
    "mev_revenue": "26014913",
    "claim_status_account": "9pdL9wRA3WW5tdJjZDeR88C8CdQ7yS8yYvfTL36hiknp"
  },
  {
    "vote_account": "3ZYJxzCeweSoh2Jj7oCgencFs9y27iKmXJeqYapje1cj",
    "mev_revenue": "248004537893",
    "claim_status_account": "3Yuwm9vEZCMcXjzd5NXcRJ9TTBnsuHDirwCbqCtPaWp8"
  },
  {
    "vote_account": "3jkJVgfz1zrHSy6YLK6g96eTj49kCnDj2i8AbbKLZhkk",
    "mev_revenue": "632365469",
    "claim_status_account": "6pTHoHApa8aP7Kv8iCNHpsmmXNs47CDrmJmdGoP99p86"
  },
  {
    "vote_account": "4T799AaK9YT7zBtVqYZEnCY5ihUF5XaEYemwr5EnozoQ",
    "mev_revenue": "535590841",
    "claim_status_account": "CoJigm6TWrsDyUCn5AW8wcmFCsevKFRgTjhzg4yLLGad"
  },
  {
    "vote_account": "4jx1b7HCN9nCxygP3hruC85BxcYndhxby4hkNexuHvxT",
    "mev_revenue": "5585858743",
    "claim_status_account": "6ZNw8U8mPRmZPm9QQAG9SjvgJV92vKYbM3rdcsFy3zm7"
  },
  {
    "vote_account": "5wYHvcKbCHPsT9dhEZaVZzoVY1qA7bosKRzKczpcaXpq",
    "mev_revenue": "1183846504",
    "claim_status_account": "EBkXrLZrA7f6PV2ALT6oAhGBiZtiZgP6XzLGpZMHuvKh"
  },
  {
    "vote_account": "6F5xdRXh2W3B2vhte12VG79JVUkUSLYrHydGX1SAadfZ",
    "mev_revenue": "41880406303",
    "claim_status_account": "CympfNrGg2RLPgDZezCs9FXntneJpxbNsn4kt4UFPoYZ"
  },
  {
    "vote_account": "6hTLQ5HSdWcpZkbXmZxXaGjCgTh7zh8UeWKWKgGE1BPp",
    "mev_revenue": "2634833850",
    "claim_status_account": "CotzKQegfXEu1Fov48YDodYoycithfReae9duy3XRMr4"
  },
  {
    "vote_account": "6jzDwKeR21EFHwaRgZMefMxJ9D2vnQRqfYxkpUuJppPh",
    "mev_revenue": "122694556881",
    "claim_status_account": "6kKiJVfZ6qWtBzazzoQ8MmDcjvRNUygvmk2AA3fx6Li5"
  },
  {
    "vote_account": "6tgtejPHUHR1pECzXqQT8EHZqnKCWZFSqdZXDyBaKe3b",
    "mev_revenue": "12179208836",
    "claim_status_account": "B7WCRb6vw1b4cK3MmsvqvrQBH9KsDx8Wu9ANfwaRrDkL"
  },
  {
    "vote_account": "72LbWsZFEyB7xrB9ggeoPUrSw2vzPEnsPJJHZo1svkM7",
    "mev_revenue": "110662983952",
    "claim_status_account": "5mvaecURvCqMCrNmHwbhEP9aGv4PoPXVCEBPM9ff9iPZ"
  },
  {
    "vote_account": "74pfDmYto6aAqCzFH1mNJ8NxF7A4LQ4cXkipGwgjY39u",
    "mev_revenue": "56477055761",
    "claim_status_account": "4dp4a5cVuTuRawYhmScWfQVmgipQrVXkVJLtTxqwPkBL"
  },
  {
    "vote_account": "7jPqpHuN5v59dtBom2tjmYEfi6WaM4sFtJeTD6fzhcdS",
    "mev_revenue": "45833923433",
    "claim_status_account": "BLoCmgwfibM2niH6LFLJfuupSCPDetVU4qbgrwn7UneL"
  },
  {
    "vote_account": "7opSZGmevWhRDyLt5Wu38FZFjUyredGmMki4DNmxDnjd",
    "mev_revenue": "33435980947",
    "claim_status_account": "rZzzuDhFqiWfUsHQrq6PyLviNG9zXk5v8gvdgpRn5rd"
  },
  {
    "vote_account": "7xENfwKCajMB5aVTgmTB6h7d7Su91wTcnfMjoAQCMvKq",
    "mev_revenue": "123415631920",
    "claim_status_account": "5WZBc63zbYtfADauCjCuPLqH3WKBtuSnzjX8CAx4ys4m"
  },
  {
    "vote_account": "8FPz3JG4E3HVXxGbPZVibarva4AGXSZWx3qKLUS5uFtN",
    "mev_revenue": "257295004",
    "claim_status_account": "BX64dYh8fBmNDV5tRsNA9ja1XELJszwHTavX3pw6ckXV"
  },
  {
    "vote_account": "8Pep3GmYiijRALqrMKpez92cxvF4YPTzoZg83uXh14pW",
    "mev_revenue": "159206649350",
    "claim_status_account": "zUeTw5JtePJS7Yh1FndP5WwyTFsbNuBUjayFtKnyUR3"
  },
  {
    "vote_account": "8mHUDJjzPo2AwJp8SHKmG9rk9ftWTp7UysqYz36cMpJe",
    "mev_revenue": "105616897844",
    "claim_status_account": "HTLbutPUWWSkigWREYos8weW1YuVX3uuRh2Y8h8JMyu8"
  },
  {
    "vote_account": "8r4Fu6M8brgnL456RJfwxk8kN4iw1LgczfuXeuG1g4px",
    "mev_revenue": "11013603288",
    "claim_status_account": "2bgxJcS9FkZ81eX14E1o2wYkcKQKJ4phoyBUug6eh2PL"
  },
  {
    "vote_account": "8wTSPukwTAzNzEYyUdc8UiKkTg1hNtZ1xLum7o1Ne6wr",
    "mev_revenue": "84672358707",
    "claim_status_account": "DUfR488SSwx7BYTjFXJVduJFMxt4F5ABkMXJEUacsq6n"
  },
  {
    "vote_account": "8zHJtME22tiY3UsSHtDJXo2J8hUfwikBxXNbVqQzA92r",
    "mev_revenue": "31332710023",
    "claim_status_account": "FRMJTioAcFgrK1sMJTwWLqQziyfUJ8HZWE1agpntHQ9V"
  },
  {
    "vote_account": "9Diao4uo6NpeMud7t5wvGnJ3WxDM7iaYxkGtJM36T4dy",
    "mev_revenue": "83286020979",
    "claim_status_account": "AdjZHsCUYVETXVbkUsSrPGY2a19v1Zc3BoNM572WoK3y"
  },
  {
    "vote_account": "9QU2QSxhb24FUX3Tu2FpczXjpK3VYrvRudywSZaM29mF",
    "mev_revenue": "267702558658",
    "claim_status_account": "Aue2bC9JYDPuCqyUJVvQ1fc8bSQ2SkfoZj37i9QYJSpb"
  },
  {
    "vote_account": "AZoCYB4VgoM9DR9f1ZFcBn8xPSbtbqoxZnKJR7tkvEoX",
    "mev_revenue": "249387620402",
    "claim_status_account": "FPjEXY1fZJ9LyCLrArfxsoamSAc7DpWqsPyKufaWSXcx"
  },
  {
    "vote_account": "At2rZHk554qWrjcmdNkCQGp8i4hdKLf52EXMrDmng5ab",
    "mev_revenue": "112907620818",
    "claim_status_account": "EWN16dERwYXxp1ShfPD4YipSPWK2oSDjYutj2oB1VHsj"
  },
  {
    "vote_account": "B38JgkTi7Fu2Uxk8JzNw4M7aMhVxzGu2fsRqHNScPkCQ",
    "mev_revenue": "141009621",
    "claim_status_account": "FRVdDkYYxf37eGsbr5MFny6YjgP5iUJvsHCmioDXn3ej"
  },
  {
    "vote_account": "BLADE1qNA1uNjRgER6DtUFf7FU3c1TWLLdpPeEcKatZ2",
    "mev_revenue": "99054608953",
    "claim_status_account": "9WZs7UJuMe8S6vwWZKvPaaUbDxCtZaZnQ2Q6yvw4EBGP"
  },
  {
    "vote_account": "BU3ZgGBXFJwNTrN6VUJ88k9SJ71SyWfBJTabYqRErm4F",
    "mev_revenue": "85235047372",
    "claim_status_account": "7dDD7tZr1voEQjg4jDjD3LKnWtFLVjPJd7DcJymrHENZ"
  },
  {
    "vote_account": "BbM5kJgrwEj3tYFfBPnjcARB54wDUHkXmLUTkazUmt2x",
    "mev_revenue": "6747153848",
    "claim_status_account": "DKFSSTTKYHeEV2fRN5PM9JWnTerFcg7yzw7uMcMjV3Ff"
  },
  {
    "vote_account": "BhREyEsP3YAtQbTCrKcXgTNTeaq9gdjWji3Nz4d8Q1P2",
    "mev_revenue": "125100235025",
    "claim_status_account": "8xDgabi7PQaDbyk3DmtQYyjsZVSC6h7JFhHX7KsbhpnH"
  },
  {
    "vote_account": "BkSS8kGUNcQkTgEKMmBhHxGVLdzw43EAzDYpZqyyxFrT",
    "mev_revenue": "58472427881",
    "claim_status_account": "CUvQMfjFXawYMqHwkTWFV5B2HhMe553jPxyvtP387rZS"
  },
  {
    "vote_account": "Bkskrv38Kn7zJR5mvmbCDGn2M4Jyhzt2ZqwQXV6rYnXa",
    "mev_revenue": "35460223302",
    "claim_status_account": "9Li19aK6ZrmRTeBkaQimCwNkjRZVSeN2FYRqqbGq4zQJ"
  },
  {
    "vote_account": "Bwkz1ddKoGE8hgiSV6HZLXi9RBLqfBi3HZb2QujzVGgz",
    "mev_revenue": "16807856393",
    "claim_status_account": "4mFZ7Aanh1j4FfNtY5ZFEYN7np4NHWhpfeQk1mv6Bayu"
  },
  {
    "vote_account": "CtiiCQbRh13cqorWaEimroRznTL2qTytNhzYz53BCnbq",
    "mev_revenue": "2849968227",
    "claim_status_account": "RVtuwrEA95htnn2VVsPZD7AtfnNbvvEjuGm8Qg5nHnH"
  },
  {
    "vote_account": "CxFH1pqJnEmyaE4wEwqdqMKpQMpkmdaMxhS7SzpHokA8",
    "mev_revenue": "52882665021",
    "claim_status_account": "CuirPq6icm4PA5FWhDZLXSiympHu62MEAp25ypZiZZ3X"
  },
  {
    "vote_account": "DLKjd8DJc9NajCaHPeQL6BnhPi3a4BZm7zCdVF3MzDRZ",
    "mev_revenue": "16988097625",
    "claim_status_account": "3bBx19EkM1aUtwCzHT19Yow9fVpeAb7PqGoqpUGKLywc"
  },
  {
    "vote_account": "DQ7D6ZRtKbBSxCcAunEkoTzQhCBKLPdzTjPRRnM6wo1f",
    "mev_revenue": "50405943955",
    "claim_status_account": "Gdkm845gU1J6WLKCeYYVixV6HSWKrgTxUuYxJ28oZBL7"
  },
  {
    "vote_account": "DdCNGDpP7qMgoAy6paFzhhak2EeyCZcgjH7ak5u5v28m",
    "mev_revenue": "503192040697",
    "claim_status_account": "GhRQYCdrJnKSqcsem7hQCq4hrA6NU6kAft2eQNTEVWe2"
  },
  {
    "vote_account": "E9W5kU2fnha9yp4RmFZgNNsRUvy6oKnB9ZyR9LC81WaE",
    "mev_revenue": "112355752842",
    "claim_status_account": "FkR1zMt45vsXpej1chHpQPYXoorE7mktWp23qDifXfGG"
  },
  {
    "vote_account": "EXhYxF25PJEHb3v5G1HY8Jn8Jm7bRjJtaxEghGrUuhQw",
    "mev_revenue": "68576440887",
    "claim_status_account": "sgz4D5wx4s6BeDbtXB5GFxLsPM78yyzJkEKNWFEyMaZ"
  },
  {
    "vote_account": "Eajfs6oXGGkvjYsxkQZZJcDCLLkUajaHizfgg2xTsqyd",
    "mev_revenue": "9588471814",
    "claim_status_account": "9Wciw6XUcUtPaoJfesSBrTR3Hgg1ZD1RbmgTXRcEHL85"
  },
  {
    "vote_account": "EcEowA4GKDsdVBF9PNAZa6c9M4WgYG8y4GnpZSUaqioS",
    "mev_revenue": "74048125",
    "claim_status_account": "5H7Tse5m2FLcPNNYUqB8kCFwbjaKQVp5DRgE2zhJd1jJ"
  },
  {
    "vote_account": "EcLPNfLFgCkbcTuvdeQ85pnQMgAfBDqi2dkoNVPrSyr5",
    "mev_revenue": "39715646052",
    "claim_status_account": "AejZVseyBzceUScpKyhCrB3sBoz3ma6Upo2JccjrUjbB"
  },
  {
    "vote_account": "EpRvips2doUUdxvs3Qhf4MCLqVeJEPu47Aci5QbBXASV",
    "mev_revenue": "4566315126",
    "claim_status_account": "C6JrbD2XVdYtyzziKva8xYZem9u631LjMR6PeaeDqwqg"
  },
  {
    "vote_account": "FjkSLYmi6BJAJQn1iSLUGrPrBQjMaD4y1DVdnv3yaTsX",
    "mev_revenue": "190390430585",
    "claim_status_account": "A9L9NCtMtQkw27GBEL43HGh3oCny5RRqaxV95qPHvNqU"
  },
  {
    "vote_account": "G9x1mqewTeVnXLmv3FamYD5tq1AdS395RHH3MLQPj6TY",
    "mev_revenue": "222655300743",
    "claim_status_account": "EQFZ2HyUm4raJPhTRBVMUn7q4iKMhQHk6PZB32SRnnp5"
  },
  {
    "vote_account": "GhBWWed6j9tXLEnKiw9CVDHyQCYunAVGnssrbYxbBmFm",
    "mev_revenue": "8449433671",
    "claim_status_account": "3NQZQSxcGhXojHtxYdW9BPJK65SzcT9MQetsSABY2Asu"
  },
  {
    "vote_account": "GioetmC79nLRnN7VDfHaq8coWAEFPJKu9py59uUqdV5U",
    "mev_revenue": "2591009461",
    "claim_status_account": "5msh2jhinq8DxVN4KXvaJoQbYGcepbsS5ZWzFs2UHZSD"
  },
  {
    "vote_account": "GvZEwtCHZ7YtCkQCaLRVEXsyVvQkRDhJhQgB6akPme1e",
    "mev_revenue": "92292661329",
    "claim_status_account": "2hfWL4sWB8P4sqjUxQNvDvLFYBF6Sw49VgvkmQQnBAJ6"
  },
  {
    "vote_account": "H43AYFsvhNuALQieHpLXefp1ECgEBT6oVnS4EcTsC25C",
    "mev_revenue": "27844669418",
    "claim_status_account": "8jHHwhRmL6DrWwC6f3hUrmKxYbzPNmpQqQP6xAqXUdBL"
  },
  {
    "vote_account": "H74qox1GASBWd94FWMyy6GVAbRVLf9SAMgbJ1tzSUAst",
    "mev_revenue": "231027486819",
    "claim_status_account": "7q8T1V9zsJ6zqf4NSuqUBj5T6zEmwa6diB6MtummwYDo"
  },
  {
    "vote_account": "HDc84gs3CtqhebHycmoDpc5n2y3CFfd5GqYZkr2XiBMR",
    "mev_revenue": "114388759230",
    "claim_status_account": "8DnE5YxdKcrGv46D8P3HMEFr8KJJPpvCFMGm8LMc7WxR"
  },
  {
    "vote_account": "HZKopZYvv8v6un2H6KUNVQCnK5zM9emKKezvqhTBSpEc",
    "mev_revenue": "209558075823",
    "claim_status_account": "F9JyM5GB3K933r9KSBbKHuG9hCQh7opyzW5s7XEebFdG"
  },
  {
    "vote_account": "HhYEE3dAShc3772wEiy73XDYnLjVxyBL8eAWKyRcF14y",
    "mev_revenue": "69801367495",
    "claim_status_account": "7AQWats4kdCHQWWf8qKCZRQNKPtFY1BfHHN2NpCAtmXe"
  },
  {
    "vote_account": "HxYHGzR58gyf6c4JAX85eK8GVuaZU2zne4be82Lq9SBQ",
    "mev_revenue": "94921236362",
    "claim_status_account": "6jZWxRgPZb6V5HcbhA7Q6K598cEi2NuqQDtn7wLLDsu1"
  },
  {
    "vote_account": "JDMq8hxZnad2smKLGkFbfg8zVMZHKQcMugD4tMR9u2da",
    "mev_revenue": "79334182045",
    "claim_status_account": "8DajTPFaoT3SKdQyBV5ciFXRve1TbN6SyDbqPd2n8VjL"
  }
]
"#;

#[derive(Error, Debug)]
pub enum ClaimMevError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    AnchorError(anchor_lang::error::Error),

    #[error(transparent)]
    RpcError(#[from] solana_rpc_client_api::client_error::Error),

    #[error("Expected to have at least {desired_balance} lamports in {payer:?}. Current balance is {start_balance} lamports. Deposit {sol_to_deposit} SOL to continue.")]
    InsufficientBalance {
        desired_balance: u64,
        payer: Pubkey,
        start_balance: u64,
        sol_to_deposit: u64,
    },

    #[error("Not finished with job, transactions left {transactions_left}")]
    NotFinished { transactions_left: usize },

    #[error("Failed to check or update completed epochs: {0}")]
    CompletedEpochsError(String),

    #[error("UncaughtError {e:?}")]
    UncaughtError { e: String },
}

#[allow(clippy::too_many_arguments)]
pub async fn emit_claim_mev_tips_metrics(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), anyhow::Error> {
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let merkle_tree_coll_path = meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch));
    let merkle_trees = GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    let rpc_url = cli.rpc_url.clone();
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        rpc_url,
        Duration::from_secs(1800),
        CommitmentConfig::confirmed(),
    );

    let epoch = merkle_trees.epoch;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    if is_epoch_completed(epoch, current_epoch, file_path, file_mutex).await? {
        return Ok(());
    }

    let all_claim_transactions = get_claim_transactions_for_valid_unclaimed(
        &rpc_client,
        &merkle_trees,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        ncn,
        0,
        Pubkey::new_unique(),
        &cli.operator_address,
        &cli.cluster,
    )
    .await?;

    datapoint_info!(
        "tip_router_cli.claim_mev_tips-send_summary",
        ("claim_transactions_left", all_claim_transactions.len(), i64),
        ("epoch", epoch, i64),
        "cluster" => &cli.cluster,
    );

    if all_claim_transactions.is_empty() {
        add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn claim_mev_tips_with_emit(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    max_loop_duration: Duration,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), anyhow::Error> {
    let keypair = read_keypair_file(cli.keypair_path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {:?}", e))?;
    let keypair = Arc::new(keypair);
    let rpc_url = cli.rpc_url.clone();
    if epoch < legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH {
        legacy_handle_claim_mev_tips(
            cli,
            epoch,
            tip_distribution_program_id,
            tip_router_program_id,
            ncn,
            max_loop_duration,
            file_path,
            file_mutex,
            &keypair,
            rpc_url,
        )
        .await?;
    } else {
        info!("In new path");
        handle_claim_mev_tips(
            cli,
            epoch,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            max_loop_duration,
            file_path,
            file_mutex,
            &keypair,
            rpc_url,
        )
        .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_claim_mev_tips(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    max_loop_duration: Duration,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    keypair: &Arc<Keypair>,
    rpc_url: String,
) -> Result<(), anyhow::Error> {
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let merkle_tree_coll_path =
        meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch) + ".tmp");
    println!("merkle_tree_coll_path: {:?}", merkle_tree_coll_path);
    let mut merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)
        .map_err(|e| anyhow::anyhow!(e))?;
    // let new_path = meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch) + ".tmp");

    // let entries: Vec<AccountAnalysisEntry> = serde_json::from_str(JSON_808)?;
    // for tree in merkle_tree_coll.generated_merkle_trees.iter_mut() {
    //     tree.tree_nodes.retain(|tree_node| {
    //         entries.iter().any(|entry| {
    //             Pubkey::from_str(&entry.claim_status_account).unwrap()
    //                 == tree_node.claim_status_pubkey
    //         })
    //     });
    // }

    // merkle_tree_coll
    //     .write_to_file(&new_path)
    //     .map_err(|e| anyhow::anyhow!(e))?;

    // return Ok(());

    // let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    // Fix wrong claim status pubkeys for 1 epoch -- noop if already correct
    // for tree in merkle_tree_coll.generated_merkle_trees.iter_mut() {
    //     if tree.merkle_root_upload_authority != tip_router_config_address {
    //         continue;
    //     }
    //     for node in tree.tree_nodes.iter_mut() {
    //         let (claim_status_pubkey, claim_status_bump) = derive_claim_status_account_address(
    //             &tree.distribution_program,
    //             &node.claimant,
    //             &tree.distribution_account,
    //         );
    //         node.claim_status_pubkey = claim_status_pubkey;
    //         node.claim_status_bump = claim_status_bump;
    //     }
    // }

    let start = Instant::now();

    match claim_mev_tips(
        &merkle_tree_coll,
        rpc_url.clone(),
        rpc_url.clone(),
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        ncn,
        keypair,
        max_loop_duration,
        cli.claim_microlamports,
        file_path,
        file_mutex,
        &cli.operator_address,
        &cli.cluster,
    )
    .await
    {
        Ok(()) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", 0, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(ClaimMevError::NotFinished { transactions_left }) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", transactions_left, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(e) => {
            datapoint_error!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("error", e.to_string(), String),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
    }

    let claimer_balance = get_claimer_balance(rpc_url, keypair).await?;
    datapoint_info!(
        "claimer_info",
        ("claimer", keypair.pubkey().to_string(), String),
        ("epoch", epoch, i64),
        ("lamport_balance", claimer_balance, i64),
        ("sol_balance", lamports_to_sol(claimer_balance), f64),
        "cluster" => &cli.cluster,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn legacy_handle_claim_mev_tips(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    max_loop_duration: Duration,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    keypair: &Arc<Keypair>,
    rpc_url: String,
) -> Result<(), anyhow::Error> {
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let merkle_tree_coll_path = meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch));
    let mut merkle_tree_coll =
        LegacyGeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)
            .map_err(|e| anyhow::anyhow!(e))?;

    let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    // Fix wrong claim status pubkeys for 1 epoch -- noop if already correct
    for tree in merkle_tree_coll.generated_merkle_trees.iter_mut() {
        if tree.merkle_root_upload_authority != tip_router_config_address {
            continue;
        }
        for node in tree.tree_nodes.iter_mut() {
            let (claim_status_pubkey, claim_status_bump) = derive_claim_status_account_address(
                &tip_distribution_program_id,
                &node.claimant,
                &tree.tip_distribution_account,
            );
            node.claim_status_pubkey = claim_status_pubkey;
            node.claim_status_bump = claim_status_bump;
        }
    }

    let start = Instant::now();

    match legacy_tip_router_operator_cli::claim::claim_mev_tips(
        &merkle_tree_coll,
        rpc_url.clone(),
        rpc_url.clone(),
        tip_distribution_program_id,
        tip_router_program_id,
        ncn,
        keypair,
        max_loop_duration,
        cli.claim_microlamports,
        file_path,
        file_mutex,
        &cli.operator_address,
        &cli.cluster,
    )
    .await
    {
        Ok(()) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", 0, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(LegacyClaimMevError::NotFinished { transactions_left }) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", transactions_left, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(e) => {
            datapoint_error!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("error", e.to_string(), String),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
    }

    let claimer_balance = get_claimer_balance(rpc_url, keypair).await?;
    datapoint_info!(
        "claimer_info",
        ("claimer", keypair.pubkey().to_string(), String),
        ("epoch", epoch, i64),
        ("lamport_balance", claimer_balance, i64),
        ("sol_balance", lamports_to_sol(claimer_balance), f64),
        "cluster" => &cli.cluster,
    );
    Ok(())
}

pub async fn get_claimer_balance(
    rpc_url: String,
    keypair: &Arc<Keypair>,
) -> Result<u64, ClaimMevError> {
    let rpc_client = RpcClient::new(rpc_url);
    let balance = rpc_client.get_balance(&keypair.pubkey()).await?;
    Ok(balance)
}

#[allow(clippy::too_many_arguments)]
pub async fn claim_mev_tips(
    merkle_trees: &GeneratedMerkleTreeCollection,
    rpc_url: String,
    rpc_sender_url: String,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    keypair: &Arc<Keypair>,
    max_loop_duration: Duration,
    micro_lamports: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    operator_address: &String,
    cluster: &str,
) -> Result<(), ClaimMevError> {
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        rpc_url,
        Duration::from_secs(1800),
        CommitmentConfig::confirmed(),
    );
    let rpc_sender_client = RpcClient::new(rpc_sender_url);

    let epoch = merkle_trees.epoch;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    if is_epoch_completed(epoch, current_epoch, file_path, file_mutex).await? {
        return Ok(());
    }

    let start = Instant::now();
    while start.elapsed() <= max_loop_duration {
        let mut all_claim_transactions = get_claim_transactions_for_valid_unclaimed(
            &rpc_client,
            merkle_trees,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            micro_lamports,
            keypair.pubkey(),
            operator_address,
            cluster,
        )
        .await?;

        datapoint_info!(
            "tip_router_cli.claim_mev_tips-send_summary",
            ("claim_transactions_left", all_claim_transactions.len(), i64),
            ("epoch", epoch, i64),
            ("operator", operator_address, String),
            "cluster" => cluster,
        );

        if all_claim_transactions.is_empty() {
            add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
            return Ok(());
        }

        // all_claim_transactions.shuffle(&mut thread_rng());

        for transactions in all_claim_transactions.chunks(1) {
            let transactions: Vec<_> = transactions.to_vec();
            // only check balance for the ones we need to currently send since reclaim rent running in parallel
            // if let Some((start_balance, desired_balance, sol_to_deposit)) =
            //     is_sufficient_balance(&keypair.pubkey(), &rpc_client, transactions.len() as u64)
            //         .await
            // {
            //     return Err(ClaimMevError::InsufficientBalance {
            //         desired_balance,
            //         payer: keypair.pubkey(),
            //         start_balance,
            //         sol_to_deposit,
            //     });
            // }

            let blockhash = rpc_client.get_latest_blockhash().await?;
            if let Err(e) = send_until_blockhash_expires(
                &rpc_client,
                &rpc_sender_client,
                transactions,
                blockhash,
                keypair,
            )
            .await
            {
                info!("send_until_blockhash_expires failed: {:?}", e);
            }
        }
    }

    let transactions = get_claim_transactions_for_valid_unclaimed(
        &rpc_client,
        merkle_trees,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        ncn,
        micro_lamports,
        keypair.pubkey(),
        operator_address,
        cluster,
    )
    .await?;
    if transactions.is_empty() {
        add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
        return Ok(());
    }

    // if more transactions left, we'll simulate them all to make sure its not an uncaught error
    let mut is_error = false;
    let mut error_str = String::new();
    for tx in &transactions {
        match rpc_client
            .simulate_transaction_with_config(
                tx,
                RpcSimulateTransactionConfig {
                    sig_verify: false,
                    replace_recent_blockhash: true,
                    commitment: Some(CommitmentConfig::processed()),
                    ..RpcSimulateTransactionConfig::default()
                },
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error_str = e.to_string();
                is_error = true;

                match e.get_transaction_error() {
                    None => {
                        break;
                    }
                    Some(e) => {
                        warn!("transaction error. tx: {:?} error: {:?}", tx, e);
                        break;
                    }
                }
            }
        }
    }

    if is_error {
        Err(ClaimMevError::UncaughtError { e: error_str })
    } else {
        info!(
            "Not finished claiming for epoch {}, transactions left {}",
            epoch,
            transactions.len()
        );
        Err(ClaimMevError::NotFinished {
            transactions_left: transactions.len(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn get_claim_transactions_for_valid_unclaimed(
    rpc_client: &RpcClient,
    merkle_trees: &GeneratedMerkleTreeCollection,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    operator_address: &String,
    cluster: &str,
) -> Result<Vec<Transaction>, ClaimMevError> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    let tree_nodes = merkle_trees
        .generated_merkle_trees
        .iter()
        .filter_map(|tree| {
            if tree.merkle_root_upload_authority != tip_router_config_address {
                return None;
            }

            Some(&tree.tree_nodes)
        })
        .flatten()
        .collect_vec();

    info!(
        "reading {} tip distribution related accounts for epoch {}",
        tree_nodes.len(),
        epoch
    );

    let start = Instant::now();

    let tda_pubkeys = merkle_trees
        .generated_merkle_trees
        .iter()
        .map(|tree| tree.distribution_account)
        .collect_vec();

    let tdas: HashMap<Pubkey, Account> = get_batched_accounts(rpc_client, &tda_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(pubkey, a)| Some((pubkey, a?)))
        .collect();

    let claimant_pubkeys = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claimant)
        .collect_vec();
    let claimants: HashMap<Pubkey, Account> = get_batched_accounts(rpc_client, &claimant_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(pubkey, a)| Some((pubkey, a?)))
        .collect();

    let claim_status_pubkeys = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claim_status_pubkey)
        .collect_vec();

    let claim_statuses: HashMap<Pubkey, Account> =
        get_batched_accounts(rpc_client, &claim_status_pubkeys)
            .await?
            .into_iter()
            .filter_map(|(pubkey, a)| Some((pubkey, a?)))
            .collect();

    let elapsed_us = start.elapsed().as_micros();

    // can be helpful for determining mismatch in state between requested and read
    datapoint_info!(
        "tip_router_cli.get_claim_transactions_account_data",
        ("elapsed_us", elapsed_us, i64),
        ("tdas", tda_pubkeys.len(), i64),
        ("tdas_onchain", tdas.len(), i64),
        ("claimants", claimant_pubkeys.len(), i64),
        ("claimants_onchain", claimants.len(), i64),
        ("claim_statuses", claim_status_pubkeys.len(), i64),
        ("claim_statuses_onchain", claim_statuses.len(), i64),
        ("epoch", epoch, i64),
        ("operator", operator_address, String),
        "cluster" => cluster,
    );

    let transactions = build_mev_claim_transactions(
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        merkle_trees,
        tdas,
        claimants,
        claim_statuses,
        micro_lamports,
        payer_pubkey,
        ncn,
        cluster,
    );

    Ok(transactions)
}

/// Returns a list of claim transactions for valid, unclaimed MEV tips
/// A valid, unclaimed transaction consists of the following:
/// - there must be lamports to claim for the tip distribution account.
/// - there must be a merkle root.
/// - the claimant (typically a stake account) must exist.
/// - the claimant (typically a stake account) must have a non-zero amount of tips to claim
/// - the claimant must have enough lamports post-claim to be rent-exempt.
///   - note: there aren't any rent exempt accounts on solana mainnet anymore.
/// - it must not have already been claimed.
#[allow(clippy::too_many_arguments)]
fn build_mev_claim_transactions(
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    merkle_trees: &GeneratedMerkleTreeCollection,
    tdas: HashMap<Pubkey, Account>,
    claimants: HashMap<Pubkey, Account>,
    claim_statuses: HashMap<Pubkey, Account>,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    ncn_address: Pubkey,
    cluster: &str,
) -> Vec<Transaction> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address =
        Config::find_program_address(&tip_router_program_id, &ncn_address).0;
    let tip_router_account_payer =
        AccountPayer::find_program_address(&tip_router_program_id, &ncn_address).0;

    let tip_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &tip_distribution_program_id).0;

    let priority_fee_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &priority_fee_distribution_program_id).0;

    let mut zero_amount_claimants = 0;

    let mut instructions = Vec::with_capacity(claimants.len());
    for tree in &merkle_trees.generated_merkle_trees {
        if tree.max_total_claim == 0 {
            continue;
        }

        // if unwrap panics, there's a bug in the merkle tree code because the merkle tree code relies on the state
        // of the chain to claim.
        let distribution_account = tdas.get(&tree.distribution_account).unwrap();
        if tree.distribution_program.eq(&tip_distribution_program_id) {
            let tda =
                TipDistributionAccount::try_deserialize(&mut distribution_account.data.as_slice());
            match tda {
                Ok(tda) => {
                    // can continue here, as there might be tip distribution accounts this account doesn't upload for
                    if tda.merkle_root.is_none()
                        || tda.merkle_root_upload_authority != tip_router_config_address
                    {
                        continue;
                    }
                }
                Err(_) => continue,
            }
        } else if tree
            .distribution_program
            .eq(&priority_fee_distribution_program_id)
        {
            let pfda = PriorityFeeDistributionAccount::try_deserialize(
                &mut distribution_account.data.as_slice(),
            );
            match pfda {
                Ok(pfda) => {
                    // can continue here, as there might be tip distribution accounts this account doesn't upload for
                    if pfda.merkle_root.is_none()
                        || pfda.merkle_root_upload_authority != tip_router_config_address
                    {
                        continue;
                    }
                }
                Err(_) => continue,
            }
        } else {
            panic!("Unknown distribution program for tree");
        }

        for node in &tree.tree_nodes {
            // doesn't make sense to claim for claimants that don't exist anymore
            // can't claim for something already claimed
            // don't need to claim for claimants that get 0 MEV
            if !claimants.contains_key(&node.claimant)
                || claim_statuses.contains_key(&node.claim_status_pubkey)
                || node.amount == 0
            {
                if node.amount == 0 {
                    zero_amount_claimants += 1;
                }
                continue;
            }

            let mut claim_with_payer_builder = ClaimWithPayerBuilder::new();
            claim_with_payer_builder
                .config(tip_router_config_address)
                .account_payer(tip_router_account_payer)
                .ncn(ncn_address)
                .tip_distribution_account(tree.distribution_account)
                .claim_status(node.claim_status_pubkey)
                .claimant(node.claimant)
                .system_program(system_program::id())
                .proof(node.proof.clone().unwrap())
                .amount(node.amount)
                .bump(node.claim_status_bump)
                .tip_distribution_program(tree.distribution_program);

            if tree.distribution_program.eq(&tip_distribution_program_id) {
                claim_with_payer_builder.tip_distribution_config(tip_distribution_config);
            } else if tree
                .distribution_program
                .eq(&priority_fee_distribution_program_id)
            {
                claim_with_payer_builder.tip_distribution_config(priority_fee_distribution_config);
            } else {
                panic!("Unknown distribution program for tree");
            }
            let claim_with_payer_ix = claim_with_payer_builder.instruction();

            instructions.push(claim_with_payer_ix);
        }
    }

    // TODO (LB): see if we can do >1 claim here
    let transactions: Vec<Transaction> = instructions
        .into_iter()
        .map(|claim_ix| {
            let instructions = priority_fees::configure_instruction(
                claim_ix,
                micro_lamports,
                Some(100_000), // helps get txs into block easier since default is 400k CUs
            );
            Transaction::new_with_payer(&instructions, Some(&payer_pubkey))
        })
        .collect();

    info!("zero amount claimants: {}", zero_amount_claimants);
    datapoint_info!(
        "tip_router_cli.build_mev_claim_transactions",
        ("distribution_accounts", tdas.len(), i64),
        ("claim_statuses", claim_statuses.len(), i64),
        ("claim_transactions", transactions.len(), i64),
        ("epoch", epoch, i64),
        "cluster" => cluster,
    );

    transactions
}

/// heuristic to make sure we have enough funds to cover the rent costs if epoch has many validators
/// If insufficient funds, returns start balance, desired balance, and amount of sol to deposit
async fn is_sufficient_balance(
    payer: &Pubkey,
    rpc_client: &RpcClient,
    instruction_count: u64,
) -> Option<(u64, u64, u64)> {
    let start_balance = rpc_client
        .get_balance(payer)
        .await
        .expect("Failed to get starting balance");
    // most amounts are for 0 lamports. had 1736 non-zero claims out of 164742
    let min_rent_per_claim = rpc_client
        .get_minimum_balance_for_rent_exemption(CLAIM_STATUS_SIZE)
        .await
        .expect("Failed to calculate min rent");
    let desired_balance = instruction_count
        .checked_mul(
            min_rent_per_claim
                .checked_add(DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE)
                .unwrap(),
        )
        .unwrap();
    if start_balance < desired_balance {
        let sol_to_deposit = desired_balance
            .checked_sub(start_balance)
            .unwrap()
            .checked_add(LAMPORTS_PER_SOL)
            .unwrap()
            .checked_sub(1)
            .unwrap()
            .checked_div(LAMPORTS_PER_SOL)
            .unwrap(); // rounds up to nearest sol
        Some((start_balance, desired_balance, sol_to_deposit))
    } else {
        None
    }
}

/// Helper function to check if an epoch is in the completed_claim_epochs.txt file
pub async fn is_epoch_completed(
    epoch: u64,
    current_epoch: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<bool, ClaimMevError> {
    // If we're still on the current epoch, it can't be completed
    let current_claim_epoch =
        current_epoch
            .checked_sub(1)
            .ok_or(ClaimMevError::CompletedEpochsError(
                "Epoch underflow".to_string(),
            ))?;

    if current_claim_epoch == epoch {
        info!("Do not skip the current claim epoch ( {} )", epoch);
        return Ok(false);
    }

    // Acquire the mutex lock before file operations
    let _lock = file_mutex.lock().await;

    // If file doesn't exist, no epochs are completed
    if !file_path.exists() {
        info!("No completed epochs file found - creating empty");
        drop(_lock);
        add_completed_epoch(0, current_epoch, file_path, file_mutex).await?;

        return Ok(false);
    }

    // Open and read file
    let file = File::open(file_path).await.map_err(|e| {
        ClaimMevError::CompletedEpochsError(format!("Failed to open completed epochs file: {}", e))
    })?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();

    // Read lines asynchronously
    while reader.read_line(&mut line).await.map_err(|e| {
        ClaimMevError::CompletedEpochsError(format!("Failed to read line from epochs file: {}", e))
    })? > 0
    {
        // Try to parse the line as a u64 and compare with our epoch
        if let Ok(completed_epoch) = line.trim().parse::<u64>() {
            if completed_epoch == epoch {
                info!("Skipping epoch {} ( already completed )", epoch);
                return Ok(true);
            }
        }

        // Clear the line for the next iteration
        line.clear();
    }

    info!("Epoch {} not found in completed epochs file", epoch);
    Ok(false)
}

/// Helper function to add an epoch to the completed_claim_epochs.txt file
pub async fn add_completed_epoch(
    epoch: u64,
    current_epoch: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), ClaimMevError> {
    // If we're still on the current epoch, it can't be completed
    let current_claim_epoch =
        current_epoch
            .checked_sub(1)
            .ok_or(ClaimMevError::CompletedEpochsError(
                "Epoch underflow".to_string(),
            ))?;

    if current_claim_epoch == epoch {
        info!("Do not write file for current epoch ( {} )", epoch);
        return Ok(());
    }

    // Acquire the mutex lock before file operations
    let _lock = file_mutex.lock().await;

    // Create or open file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .map_err(|e| {
            ClaimMevError::CompletedEpochsError(format!(
                "Failed to open epochs file for writing: {}",
                e
            ))
        })?;

    // Write epoch followed by newline
    file.write_all(format!("{}\n", epoch).as_bytes())
        .await
        .map_err(|e| {
            ClaimMevError::CompletedEpochsError(format!("Failed to write epoch to file: {}", e))
        })?;

    info!(
        "Epoch {} added to completed epochs file ( {} )",
        epoch,
        file_path.display()
    );
    Ok(())
}
