import * as fs from "fs";

function writeCachedContractChecksums(cachedContractChecksums) {
  fs.writeFileSync(
    `cache/contractChecksums_${process.env.NETWORK_ENV}.json`,
    JSON.stringify(cachedContractChecksums),
    "utf8"
  );
}

export { writeCachedContractChecksums };
