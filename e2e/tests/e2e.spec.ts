import { expect } from "chai";

import { before } from "mocha";

import { createClient } from "../lib/createClient.js";
import { createUser } from "../lib/createUser.js";

import { readContractAddrs } from "../lib/readContractAddrs.js";
import { readCodeIds } from "../lib/readCodeIds.js";
import { getAttribute } from "../lib/getAttribute.js"
import { toBase64 } from "../lib/toBase64.js"
import { sleep } from "../lib/sleep.js"

import { IdentityserviceClient } from "../client/Identityservice.client.js";
import { DaoMultisigQueryClient, DaoMultisigClient } from "../client/DaoMultisig.client.js";
import { DaoMembersQueryClient } from "../client/DaoMembers.client.js";
import * as DaoMultisig from "../client/DaoMultisig.types";
import * as DaoMembers from "../client/DaoMembers.types";
import { GovernanceClient, GovernanceQueryClient } from "../client/Governance.client.js";
import * as Governance from "../client/Governance.types.js";
import { Core } from "jmes";
import { WasmMsg } from "jmes/src/Client/providers/LCDClient/core/wasm/msgs";
import { coin, coins } from "@cosmjs/amino";
import { useGovernanceCoreSlotsQuery } from "client/Governance.react-query.js";
import { govTypes } from "@cosmjs/stargate/build/modules/index.js";
import { SigningCosmWasmClient, CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { getOfflineSignerProto } from "cosmjs-utils";
import { GasPrice } from "@cosmjs/stargate";

// const wasmMsg = Core.Msg.fromData(data);

const client = (await createClient()) as any;

const queryClient = await CosmWasmClient.connect(process.env.RPCURL);


const user1 = createUser(process.env.USER1_MNEMONIC);
const user1_name = process.env.USER1_NAME;

const user2 = createUser(process.env.USER2_MNEMONIC);
const user2_name = process.env.USER2_NAME;

const user3 = createUser(process.env.USER3_MNEMONIC);
const user3_name = process.env.USER3_NAME;

const user4 = createUser(process.env.USER4_MNEMONIC);
const user4_name = process.env.USER4_NAME;

global.liveAddrs = {};

describe("End-to-End Tests", function () {
  describe("User Identity", function () {
    before(async function () {
      global.addrs = await readContractAddrs();
    });
    it("should register a user identity with valid name", async function () {
      const contract_addr = global.addrs.identityservice;

      const identityClient = new IdentityserviceClient(client, user1, contract_addr);
      const result = await identityClient.registerUser({ name: user1_name });

      console.log('result :>> ', result);

      expect(result['code']).to.equal(0);

      return result;
    });
  });
  describe("DAO Identity", function () {
    before(async function () {
      global.addrs = readContractAddrs();
      global.codeIds = readCodeIds();
      console.log('global.addrs :>> ', global.addrs);
      console.log('global.codeIds :>> ', global.codeIds);
    });
    it("should register a dao identity with valid name", async function () {
      const contract_addr = global.addrs.identityservice;
      const identityClient = new IdentityserviceClient(client, user2, contract_addr);

      try {
        const result = await identityClient.registerDao({
          daoName: user2_name,
          members: [
            {
              addr: user1.address,
              weight: 33,
            },
            {
              addr: user2.address,
              weight: 33,
            },
            {
              addr: user4.address,
              weight: 34,
            },
          ],
          thresholdPercentage: 51,
          maxVotingPeriod: {
            height: 1180000,
          },
        });

        // TODO use binary response to read contract address
        global.liveAddrs.dao_members = result.logs[0]['eventsByType'].instantiate._contract_address[0];
        global.liveAddrs.dao_multisig = result.logs[0]['eventsByType'].instantiate._contract_address[1];

        expect(result['code']).to.equal(0);

        console.log('user1.address :>> ', user1.address);
        console.log('global.liveAddrs.dao_multisig :>> ', global.liveAddrs.dao_multisig);

        return result;
      } catch (e) {
        console.error(e)
        throw e
      }


    });
  });

  describe.skip("DAO Proposal", function () {
    before(async function () {
      // global.liveAddrs = { dao_multisig: "jmes1wr5uxeez5h3qkpxwsrmwmarfcknajytvw8fvzjr4jyduykftp7xscps7gr" }
      client.send(user3, global.liveAddrs.dao_multisig, "1000ujmes")
    });
    describe("send dao tokens", function () {
      it("should create a dao proposal: send tokens", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user1, contractAddress);

        const coin: DaoMultisig.Coin = { denom: "ujmes", amount: "1000" }
        const bankMsg: DaoMultisig.BankMsg = { send: { amount: [coin], to_address: user2.address } }

        const msg: DaoMultisig.ExecuteMsg = {
          propose: {
            title: "Funds withdrawal",
            description: "Spend 1000 coins",
            msgs: [
              {
                bank: bankMsg
              },
            ],
          }
        };

        const result = await daoClient.propose(msg.propose);

        this.dao_send_token_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should vote on a dao proposal: send tokens", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user2, contractAddress);

        const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should execute on a passed dao proposal: send tokens", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user1, contractAddress);

        const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });

        global.governance_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      });
    });
    describe.skip("update dao members", function () { // FIX      TypeError: this.client.queryContractSmart is not a function
      it("should create a dao proposal: updatemembers", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoMultisigClient = new DaoMultisigClient(client, user1, contractAddress);

        const dao_members_addr = (await daoMultisigClient.config()).dao_members_addr;

        const updateMembersMsg: DaoMembers.ExecuteMsg = {
          update_members: {
            add: [{ addr: user1.address, weight: 25 }, { addr: user3.address, weight: 27 }],
            remove: [user2.address],
          }
        };

        const wasmMsg: Governance.WasmMsg = {
          execute: {
            contract_addr: dao_members_addr,
            funds: [],
            msg: toBase64(updateMembersMsg)
          }
        }

        const msg: DaoMultisig.ExecuteMsg = {
          propose: {
            title: "UpdateMembers",
            description: "Add user3, remove user2",
            msgs: [
              {
                wasm: wasmMsg
              },
            ],
          }
        };

        const result = await daoMultisigClient.propose(msg.propose);

        this.dao_send_token_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should vote on a dao proposal: update members", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user2, contractAddress);

        const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should execute on a passed dao proposal: update members", async function () {
        try {
          const contractAddress = global.liveAddrs.dao_multisig
          const daoMultisigClient = new DaoMultisigClient(client, user1, contractAddress);

          const result = await daoMultisigClient.execute({ proposalId: this.dao_send_token_proposal_id });

          global.governance_proposal_id = parseInt(
            getAttribute(result, "wasm", "proposal_id")
          );

          expect(result['code']).to.equal(0);
          return result;
        } catch (e) {
          console.error(e)
          throw e
        }
      });
      it("should fetch the new members", async function () {
        try {
          const daoMultisigAddr = global.liveAddrs.dao_multisig
          const daoMultisigConfig = await new DaoMultisigQueryClient(client, daoMultisigAddr).config();

          const daoMembersAddr = daoMultisigConfig.dao_members_addr;
          const daoMembersQueryClient = new DaoMembersQueryClient(client, daoMembersAddr);

          const result = await daoMembersQueryClient.listMembers({ limit: 10, startAfter: null });

          expect(result).to.deep.equal(
            {
              members: [
                {
                  addr: 'terra1757tkx08n0cqrw7p86ny9lnxsqeth0wgp0em95',
                  weight: 27
                },
                {
                  addr: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
                  weight: 25
                }
              ]
            });

          return result;
        } catch (e) {
          console.error(e)
          throw e
        }
      });
    });
  });

  describe.skip("Governance Funding Proposal", function () {
    before(async function () {
      global.addrs = await readContractAddrs();

      // Mint bondedJMES token so we can vote
      // TODO use native ubjmes 
      // await bjmesTokenClient.mint({ amount: "4000", recipient: user1.address }) // Token for voting
    });



    // it("should create a dao proposal: Governance Funding", async function () {
    //   const contractAddress = global.liveAddrs.dao_multisig
    //   const daoClient = new DaoMultisigClient(client, user1, contractAddress);

    //   // Governance Proposal Msg
    //   const proposalMsg: Governance.ExecuteMsg = {
    //     propose: {
    //       funding: {
    //         title: "Funding",
    //         description: "Give me money",
    //         amount: "1000000",
    //         duration: 300,

    //       }
    //     }

    //   };

    //   const deposit: Governance.Coin = { denom: "ujmes", amount: "1000" }

    //   const wasmMsg: Governance.WasmMsg = {
    //     execute: {
    //       contract_addr: global.addrs.governance,
    //       funds: [deposit],
    //       msg: toBase64(proposalMsg)
    //     }
    //   }

    //   // Dao Proposal Msg (Executes the bondedJMES (cw20) Send Msg)
    //   const msg: DaoMultisig.ExecuteMsg = {
    //     propose: {
    //       title: "Request Funding from Governance",
    //       description: "Make us rich",
    //       msgs: [
    //         {
    //           wasm: wasmMsg
    //         },
    //       ],
    //     }
    //   };

    //   try {
    //     const result = await daoClient.propose(msg.propose);

    //     this.dao_send_token_proposal_id = parseInt(
    //       getAttribute(result, "wasm", "proposal_id")
    //     );

    //     expect(result['code']).to.equal(0);
    //     return result;
    //   } catch (e) {
    //     console.error(e)
    //     throw e
    //   }

    // });
    it("should vote on a dao proposal: Governance Funding", async function () {
      const contractAddress = global.liveAddrs.dao_multisig
      const daoClient = new DaoMultisigClient(client, user2, contractAddress);

      const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

      expect(result['code']).to.equal(0);
      return result;
    });
    it("should execute on a passed dao proposal: Governance Funding", async function () {
      const contractAddress = global.liveAddrs.dao_multisig
      const daoClient = new DaoMultisigClient(client, user1, contractAddress);
      try {
        const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });

        global.governance_proposal_id = parseInt(
          getAttribute(result, "wasm", "proposal_id")
        );

        expect(result['code']).to.equal(0);
        return result;
      } catch (e) {
        console.error(e)
        throw e
      }
    });
    it("should return the current governance period as: posting", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      const periodInfo = await governanceClient.periodInfo()

      // console.log('periodInfo :>> ', periodInfo);
      expect(periodInfo.current_period).to.eq('posting')
      return periodInfo
    })
    it("should return the current governance period as: voting", async function () {
      await sleep(20000);
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);

      const periodInfo = await governanceClient.periodInfo()

      // console.log('periodInfo :>> ', periodInfo);
      expect(periodInfo.current_period).to.eq('voting')
      return periodInfo
    })
    it("should vote 'yes' as user1", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      try {

        const result = await governanceClient.vote({ id: 1, vote: "yes" })

        // console.log('result :>> ', result);

        return result
      } catch (e) {
        console.error(e)
        throw e
      }
    })
    it("should fetch the proposal", async function () {
      const governanceQueryClient = new GovernanceQueryClient(client, global.addrs.governance);
      const result = await governanceQueryClient.proposal({ id: 1 })

      // console.log('proposal result :>> ', result);
      return result
    });

    it("should conclude the proposal", async function () {
      const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
      await sleep(30000);

      const result = await governanceClient.conclude({ id: 1 })

      // console.log('result :>> ', result);
      return result
    })
  });
  describe("Governance Coreslot and Improvement Proposal ", function () {
    before(async function () {
      global.addrs = await readContractAddrs();
      // global.liveAddrs.dao_multisig = "jmes10ly5g36gmtlgw68wlmxcu79y3lxuff8jfqps0zfq8mlx9qfc7qdq8d3jdt";
      await client.send(user1, global.liveAddrs.dao_multisig, "20000000ujmes")

      console.log('global.liveAddrs.dao_multisig :>> ', global.liveAddrs.dao_multisig);

      // Fund the governance contract to test improvement:BankMsg
      await client.send(user1, global.addrs.governance, "300000ujmes")

      const signer = await getOfflineSignerProto({
        mnemonic: process.env.USER1_MNEMONIC,
        chain: {
          bech32_prefix: "jmes",
          slip44: 6280,
        },
      });

      const signingClient = await SigningCosmWasmClient.connectWithSigner(
        process.env.RPCURL,
        signer,
        { gasPrice: GasPrice.fromString("0.3ujmes") }
      );

      // Delegate tokens to the validator so we can vote with bondedJMES (voting rights)
      // const result = await signingClient.delegateTokens((await signer.getAccounts())[0].address, process.env.VALIDATOR, coin("1000000", "ujmes"), "auto")
    });


    describe("CoreSlot Proposal ", function () {

      it("should create a dao proposal: Governance CoreSlot: CoreTech", async function () {
        const daoMultisigAddress = global.liveAddrs.dao_multisig
        console.log('global.liveAddrs.dao_multisig :>> ', global.liveAddrs.dao_multisig);
        console.log('daoMultisigAddress :>> ', daoMultisigAddress);
        const daoClient = new DaoMultisigClient(client, user1, daoMultisigAddress);
        const slot: Governance.CoreSlot = { core_tech: {} };
        // Governance Proposal Msg
        const proposalMsg: Governance.ExecuteMsg = {
          propose: {
            core_slot: {
              title: "Make me CoreTech",
              description: "Serving the Chain",
              slot,
              funding: {
                amount: "1000000000",
                duration_in_blocks: 485700
              }

            }
          }
        };

        const proposalFee: Governance.Coin = { denom: "ujmes", amount: "10000000" }

        const wasmMsg: Governance.WasmMsg = {
          execute: {
            contract_addr: global.addrs.governance,
            funds: [proposalFee],
            msg: toBase64(proposalMsg)
          }
        }

        console.log('wasmMsg :>> ', wasmMsg);

        // Dao Proposal Msg
        const msg: DaoMultisig.ExecuteMsg = {
          propose: {
            title: "Make me CoreTech",
            description: "Serving the Chain",
            msgs: [
              {
                wasm: wasmMsg
              },
            ],
          }
        };

        try {
          const result = await daoClient.propose(
            msg.propose,
            "auto",
            undefined,
            [proposalFee] //FIXME _funds is not used; Fund the DAO so it can pay the proposal fee to the governance contract

          );

          console.log('result did it send :>> ', result);

          this.dao_send_token_proposal_id = parseInt(
            getAttribute(result, "wasm", "proposal_id")
          );

          expect(result['code']).to.equal(0);
          return result;
        } catch (e) {
          console.error(e)
          throw e
        }

      });
      it("should vote on a dao proposal: Governance CoreSlot: CoreTech", async function () {

        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user2, contractAddress);

        const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should execute on a passed dao proposal: Governance CoreSlot: CoreTech", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user1, contractAddress);
        try {
          const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });

          global.governance_proposal_id = parseInt(
            getAttribute(result, "wasm", "proposal_id")
          );

          expect(result['code']).to.equal(0);
          return result;
        } catch (e) {
          console.error(e)
          throw e
        }
      });
      it("should return the current governance period as: posting", async function () {
        const governanceClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        const periodInfo = await governanceClient.periodInfo()

        // console.log('periodInfo :>> ', periodInfo);
        expect(periodInfo.current_period).to.eq('posting')
        const timeToVotingPeriod = periodInfo.posting_period_length - periodInfo.current_time_in_cycle + 10;
        console.log('periodInfo :>> ', periodInfo);
        console.log('timeToVotingPeriod :>> ', timeToVotingPeriod);
        await sleep(timeToVotingPeriod * 1000);
        return periodInfo
      })
      it("should return the current governance period as: voting", async function () {
        // await sleep(17000);
        const governanceClient = new GovernanceQueryClient(queryClient, global.addrs.governance);

        const periodInfo = await governanceClient.periodInfo()

        console.log('periodInfo :>> ', periodInfo);
        expect(periodInfo.current_period).to.eq('voting')
        return periodInfo
      })
      it("should vote 'yes' as user4", async function () {
        const governanceClient = new GovernanceClient(client, user4, global.addrs.governance);
        try {

          const result = await governanceClient.vote({ id: 1, vote: "yes" })

          // console.log('result :>> ', result);

          return result
        } catch (e) {
          console.error(e)
          throw e
        }
      })
      it("should fetch the proposal", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        const result = await governanceQueryClient.proposal({ id: 1 })

        console.log('proposal result :>> ', result);
        return result
      });

      it("should conclude the proposal", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);

        let periodInfo = await governanceQueryClient.periodInfo();
        console.log('periodinfo :>> ', periodInfo);

        const timeToPostingPeriod = periodInfo.cycle_length - periodInfo.current_time_in_cycle + 10;
        console.log('timeToPostingPeriod :>> ', timeToPostingPeriod);
        await sleep(timeToPostingPeriod * 1000);

        periodInfo = await governanceQueryClient.periodInfo();
        console.log('periodinfo :>> ', periodInfo);

        const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);
        const result = await governanceClient.conclude({ id: 1 })

        console.log('conclude result :>> ', result);
        console.dir(result, { depth: 100 })
        return result
      })
      // it should fetch the core slots
      it("should fetch the core slots, `the dao` should hold the core_tech slot", async function () {
        await sleep(10000);
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);

        const proposalresult = await governanceQueryClient.proposal({ id: 1 })

        console.log('proposal result :>> ', proposalresult);


        const result = await governanceQueryClient.coreSlots()
        console.log('coreSlots result :>> ', result);

        expect(result.core_tech.dao).to.eq(global.liveAddrs.dao_multisig);
        return result
      });
    });
    describe("Improvement Proposal ", function () {

      it("should create a dao proposal: Governance Improvement", async function () {
        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user1, contractAddress);
        const slot: Governance.CoreSlot = { core_tech: {} };
        // Governance Proposal Msg
        const proposalMsg: Governance.ExecuteMsg = {
          propose: {
            improvement: {
              title: "Send Funds",
              description: "Improvement BankMsg",
              msgs: [
                {
                  bank: { send: { amount: [{ denom: "ujmes", amount: "1000" }], to_address: user1.address } }
                },
                // {
                //   wakm: { migrate: { contract_addr: global.addrs.governance, msg: toBase64({}), new_code_id: 27 } }
                // }
              ]
            }
          }
        };

        const proposalFee: Governance.Coin = { denom: "ujmes", amount: "10000000" }

        const wasmMsg: Governance.WasmMsg = {
          execute: {
            contract_addr: global.addrs.governance,
            funds: [proposalFee],
            msg: toBase64(proposalMsg)
          }
        }

        console.log('wasmMsg :>> ', wasmMsg);

        // Dao Proposal Msg (Executes the bondedJMES (cw20) Send Msg)
        const msg: DaoMultisig.ExecuteMsg = {
          propose: {
            title: "Send Funds",
            description: "Improvement BankMsg",
            msgs: [
              {
                wasm: wasmMsg
              },
            ],
          }
        };

        try {
          const result = await daoClient.propose(msg.propose);

          this.dao_send_token_proposal_id = parseInt(
            getAttribute(result, "wasm", "proposal_id")
          );

          expect(result['code']).to.equal(0);
          return result;
        } catch (e) {
          console.error(e)
          throw e
        }

      });
      it("should vote on a dao proposal: Governance Improvement", async function () {

        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user2, contractAddress);

        const result = await daoClient.vote({ proposalId: this.dao_send_token_proposal_id, vote: "yes" });

        expect(result['code']).to.equal(0);
        return result;
      });
      it("should execute on a passed dao proposal: Governance Improvement", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        let periodInfo = await governanceQueryClient.periodInfo();

        const timeToPostingPeriod = periodInfo.cycle_length - periodInfo.current_time_in_cycle + 10;
        console.log('timeToPostingPeriod :>> ', timeToPostingPeriod);
        await sleep(timeToPostingPeriod * 1000);

        periodInfo = await governanceQueryClient.periodInfo();
        console.log('periodinfo :>> ', periodInfo);

        const contractAddress = global.liveAddrs.dao_multisig
        const daoClient = new DaoMultisigClient(client, user1, contractAddress);
        try {
          const result = await daoClient.execute({ proposalId: this.dao_send_token_proposal_id });
          expect(result['code']).to.equal(0);
          return result;
        } catch (e) {
          console.error(e)
          throw e
        }
      });
      it("should list the governance proposals", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        const proposals = await governanceQueryClient.proposals({})
        console.log('proposals :>> ', proposals);
        global.governance_proposal_id = proposals.proposals[0].id
        console.log('global.governance_proposal_id :>> ', global.governance_proposal_id);

      });
      it("should return the current governance period as: posting", async function () {
        const governanceClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        const periodInfo = await governanceClient.periodInfo()
        console.log('periodInfo :>> ', periodInfo);

        expect(periodInfo.current_period).to.eq('posting')
        // await sleep(20000);
        return periodInfo
      })
      it("should return the current governance period as: voting", async function () {
        const governanceClient = new GovernanceQueryClient(queryClient, global.addrs.governance);

        let periodInfo = await governanceClient.periodInfo()
        console.log('periodInfo :>> ', periodInfo);

        const timeToVotingPeriod = periodInfo.posting_period_length - periodInfo.current_time_in_cycle + 10;
        console.log('timeToVotingPeriod :>> ', timeToVotingPeriod);
        await sleep(timeToVotingPeriod * 1000);

        periodInfo = await governanceClient.periodInfo()
        console.log('periodInfo :>> ', periodInfo);

        expect(periodInfo.current_period).to.eq('voting')
        return periodInfo
      })
      it("should vote 'yes' as user4", async function () {
        const governanceClient = new GovernanceClient(client, user4, global.addrs.governance);
        try {
          console.log('global.governance_proposal_id :>> ', global.governance_proposal_id);
          const result = await governanceClient.vote({ id: global.governance_proposal_id, vote: "yes" })

          // console.log('result :>> ', result);

          return result
        } catch (e) {
          console.log('global.governance_proposal_id :>> ', global.governance_proposal_id);

          console.error(e)
          throw e
        }
      })
      it("should fetch the proposal", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);
        const result = await governanceQueryClient.proposal({ id: global.governance_proposal_id })

        // console.log('proposal result :>> ', result);
        return result
      });

      it("should conclude the proposal", async function () {
        const governanceQueryClient = new GovernanceQueryClient(queryClient, global.addrs.governance);

        let periodInfo = await governanceQueryClient.periodInfo();

        const timeToPostingPeriod = periodInfo.cycle_length - periodInfo.current_time_in_cycle + 10;
        console.log('timeToPostingPeriod :>> ', timeToPostingPeriod);
        await sleep(timeToPostingPeriod * 1000);

        periodInfo = await governanceQueryClient.periodInfo();
        console.log('periodinfo :>> ', periodInfo);

        try {
          const governanceClient = new GovernanceClient(client, user1, global.addrs.governance);

          const result = await governanceClient.conclude({ id: global.governance_proposal_id })

          // console.log('result :>> ', result);
          return result
        } catch (e) {
          console.error(e)
          throw e
        }

      })
    });
  });
});