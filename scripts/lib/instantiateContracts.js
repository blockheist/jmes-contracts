import {
  MsgExecuteContract,
  MsgInstantiateContract,
  MsgUpdateContractAdmin,
} from "jmes/build/Client/providers/LCDClient/core/index.js";
import { writeFileSync } from "fs";
import { executeMsg } from "./executeMsg.js";
import { getAttribute } from "../../e2e/lib/getAttribute.js";
import { readContractAddrs } from "../../e2e/lib/readContractAddrs.js";
import { readCodeIds } from "../../e2e/lib/readCodeIds.js";
import { sleep } from "./sleep.js";

function storeContractAddr(contractName, contractAddr) {
  let contractAddrs = readContractAddrs();

  contractAddrs[contractName] = contractAddr;

  writeFileSync(
    `configs/contractAddrs_${process.env.NETWORK_ENV}.json`,
    JSON.stringify(contractAddrs),
    "utf8"
  );
}

async function instantiateContract(
  client,
  user,
  contractName,
  codeId,
  initMsg,
  options = {}
) {
  // Hydrate contract addrs for value starting with __
  for (const [key, value] of Object.entries(initMsg)) {
    if (typeof value === "string" && value.slice(0, 2) == "__") {
      const contractAddrs = readContractAddrs();
      initMsg[key] = contractAddrs[value.slice(2, value.length)];
    }
  }

  // console.log("initMsg:>> ", initMsg);

  // The governance contract is instantiated with a temporary admin address (and set to its own contract address later)
  // This will be superseded with MsgInstantiate2 in cosmwasm 1.2
  // All other contracts are instantiated with the governance contract address as admin
  const admin_address =
    contractName === "governance"
      ? process.env.ADMIN
      : readContractAddrs()["governance"];

  console.log("admin_address :>> ", admin_address);

  const instantiateContractMsg = new MsgInstantiateContract(
    user.address,
    admin_address,
    codeId,
    initMsg,
    {},
    contractName
  );

  // console.log("instantiateContractMsg :>> ", instantiateContractMsg);
  let result;
  try {
    result = await executeMsg(client, instantiateContractMsg, user.wallet);
    // console.log("result instantiateContractMsg :>> ", result, user.address); // If this fails make sure your wallet has ujmes to pay the fees
  } catch (err) {
    console.log(err);
    throw err;
  }
  const contractAddr = getAttribute(result, "instantiate", "_contract_address");
  console.log(`-> Instantiated ${contractName}:`, contractAddr);

  // Set the governance admin address to its own contract address
  if (contractName === "governance") {
    const updateContractAdminMsg = new MsgUpdateContractAdmin(
      // @ts-ignore
      process.env.ADMIN,
      contractAddr,
      contractAddr
    );
    let updateAdminResult;
    try {
      updateAdminResult = await executeMsg(
        client,
        updateContractAdminMsg,
        user.wallet
      );
      console.log("updateAdminResult :>> ", updateAdminResult);
    } catch (err) {
      console.log(err);
      throw err;
    }
  }

  if (options.cache) storeContractAddr(contractName, contractAddr);

  return result;
}

async function instantiateContracts(client, user, options = {}) {
  const codeIds = readCodeIds();

  const instantiateMsgs = [
    {
      governance: {
        owner: process.env.OWNER, // only used once for set_contract
        proposal_required_deposit: "10000000", // 10_000_000 ujmes
        proposal_required_percentage: 10, // 10% more net yes votes than no votes
        period_start_epoch: Math.floor(Date.now() / 1000), // 1660000000,
        posting_period_length: 70, // 70 seconds
        voting_period_length: 20, // 20 seconds
      },
    },
    {
      identityservice: {
        owner: "__governance", // __ gets hydrated with governance contract addr
        dao_members_code_id: codeIds["dao_members"],
        dao_multisig_code_id: codeIds["dao_multisig"],
        governance_addr: "__governance", // __ gets hydrated with governance contract addr
      },
    },
    {
      art_dealer: {
        owner: "__governance", // __ gets hydrated with governance contract addr
        identityservice_contract: "__identityservice", // __ gets hydrated with identityservice contract addr
        art_nft_name: "Art NFT",
        art_nft_symbol: "artnft",
        art_nft_code_id: codeIds["cw721_metadata_onchain"],
      },
    },
  ];

  console.log(
    "-> Instantiate Contracts in order of provided initMsg as",
    user.address
  );

  for (let idx = 0; idx < instantiateMsgs.length; idx++) {
    const [contractName, initMsg] = Object.entries(instantiateMsgs[idx])[0];
    const codeId = codeIds[contractName];

    console.log(`-> Instantiate ${contractName} with codeId ${codeId}`);
    const result = await instantiateContract(
      client,
      user,
      contractName,
      codeId,
      initMsg,
      options
    );
    await sleep(2000);
  }

  const contractAddrs = readContractAddrs();

  const setContractsMsg = new MsgExecuteContract(
    user.address,
    contractAddrs.governance,
    {
      set_contract: {
        art_dealer: contractAddrs.art_dealer,
        identityservice: contractAddrs.identityservice,
      },
    }
  );

  console.log("setContractsMsg :>> ", setContractsMsg);

  const result = await executeMsg(client, setContractsMsg, user.wallet);

  // console.log("result :>> ", result);

  console.log("contractAddrs :>> ", readContractAddrs());

  return readContractAddrs();
}

export { instantiateContracts };
