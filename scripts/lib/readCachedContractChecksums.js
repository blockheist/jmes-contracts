import * as fs from "fs";

function readCachedContractChecksums() {
  let cachedContractChecksums = {};

  try {
    cachedContractChecksums = JSON.parse(
      fs.readFileSync(
        `cache/contractChecksums_${process.env.NETWORK_ENV}.json`,
        "utf8"
      )
    );
  } catch (e) {
    console.log("-> No cached contract checksums found..");
  }
  return cachedContractChecksums;
}

export { readCachedContractChecksums };
