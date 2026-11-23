# PR Commit Recovery and Integration Matrix

**Repository:** `Mujshanan/forma-deform`  
**Snapshot:** 2026-11-23  
**Current `main` HEAD:** `7ae59f4f51da19b0e2160deed966ff5da8870903`  
**Canonical review PR:** [#83](https://github.com/Mujshanan/forma-deform/pull/83)  
**Scope:** PRs #1–#85

## 1. Verified disposition

| Disposition | Count | PRs |
|---|---:|---|
| Merged directly into `main` | 7 | #1, #2, #10, #11, #18, #84, #85 |
| Merged into non-main feature branches | 8 | #9, #38, #48, #50, #51, #52, #54, #56 |
| Closed without merging | 69 | All remaining closed PRs |
| Open | 1 | #83 |

A PR's GitHub `merged_at` value and its recorded base branch determine whether it merged to `main` or only to another branch. A feature-branch merge is not evidence of integration into `main`.

## 2. Per-PR commit and diff metadata

The base SHA is the base tip recorded for the PR. The head SHA is the currently observed head ref at the time of this snapshot. A head branch can be reused as another PR's base and advance after the earlier PR was closed; therefore the current head SHA is not always the original review-time head. For merged PRs, the merge commit SHA is the authoritative integration point for that PR's recorded base.

| PR | Disposition | Base ref | Base SHA | Head ref | Observed head SHA | Merge commit SHA | Changed files (+/−) |
|---:|---|---|---|---|---|---|---|
| [#1](https://github.com/Mujshanan/forma-deform/pull/1) | Merged → main | `main` | `4c436ee1291bc37eba00068d0a61072eb650c536` | `feat/rectangle-deformation-mvp` | `631ead7ea9e3ad1c575fbaccba8a6fbdda97b13d` | `631ead7ea9e3ad1c575fbaccba8a6fbdda97b13d` | — (+0/−0) |
| [#2](https://github.com/Mujshanan/forma-deform/pull/2) | Merged → main | `main` | `4c436ee1291bc37eba00068d0a61072eb650c536` | `feat/smart-controller-v1` | `0f2c45670489cc55d4dce78f305687aa1003d5f5` | `6b53aac51ddc15249c5100745726027fd9c75907` | — (+0/−0) |
| [#3](https://github.com/Mujshanan/forma-deform/pull/3) | Closed, unmerged | `main` | `6b53aac51ddc15249c5100745726027fd9c75907` | `feat/v4-mcp-slider-prototype` | `c83cde7023b4e9a75c0af618ce9f0e402e54deed` | `—` | — (+0/−0) |
| [#4](https://github.com/Mujshanan/forma-deform/pull/4) | Closed, unmerged | `main` | `6b53aac51ddc15249c5100745726027fd9c75907` | `refactor/ccc-01-3-shared-primitives` | `2ad8fef284bbe66e77a6ae221fdcaf8a2a7475b9` | `—` | — (+0/−0) |
| [#5](https://github.com/Mujshanan/forma-deform/pull/5) | Closed, unmerged | `refactor/ccc-01-3-shared-primitives` | `2ad8fef284bbe66e77a6ae221fdcaf8a2a7475b9` | `refactor/ccc-01-4-controller-consolidation` | `2ad6061c6fe2225e49a356a377b74746eb0523f9` | `—` | — (+0/−0) |
| [#6](https://github.com/Mujshanan/forma-deform/pull/6) | Closed, unmerged | `refactor/ccc-01-4-controller-consolidation` | `2ad6061c6fe2225e49a356a377b74746eb0523f9` | `refactor/ccc-01-5-deformation-integration` | `3a3b13e60d32e6a8b46ece84aee57bd507b1f958` | `—` | — (+0/−0) |
| [#7](https://github.com/Mujshanan/forma-deform/pull/7) | Closed, unmerged | `refactor/ccc-01-5-deformation-integration` | `3a3b13e60d32e6a8b46ece84aee57bd507b1f958` | `refactor/ccc-01-6-adapter-isolation` | `af69849c30493595b51cc413289067aa011b1c83` | `—` | — (+0/−0) |
| [#8](https://github.com/Mujshanan/forma-deform/pull/8) | Closed, unmerged | `refactor/ccc-01-6-adapter-isolation` | `af69849c30493595b51cc413289067aa011b1c83` | `test/ccc-01-7-compatibility-verification` | `89103e2527135ccfbe479102f1a30702ba59de4d` | `—` | — (+0/−0) |
| [#9](https://github.com/Mujshanan/forma-deform/pull/9) | Merged → feature branch (not main) | `test/ccc-01-7-compatibility-verification` | `d610a52334ca4b9a2c3593589bb1c01be174e05f` | `docs/ccc-01-8-fdm-v2-readiness` | `04186874e36ae336c233595d803f98a8ad208a36` | `89103e2527135ccfbe479102f1a30702ba59de4d` | — (+0/−0) |
| [#10](https://github.com/Mujshanan/forma-deform/pull/10) | Merged → main | `main` | `6b53aac51ddc15249c5100745726027fd9c75907` | `feat/smart-controller-phase1-foundation` | `339d09c93964483b04641e59dcaf905c9cfa0af8` | `9e5b57f08fbd6f5b25de8ec1bb401692b1e4749b` | — (+0/−0) |
| [#11](https://github.com/Mujshanan/forma-deform/pull/11) | Merged → main | `main` | `9e5b57f08fbd6f5b25de8ec1bb401692b1e4749b` | `feat/smart-controller-phase3-evaluator` | `4b7242cc85b2209e8d0d9ef0b1212636c9b12e28` | `9003088254d7301522eef474ff560db2eca81860` | — (+0/−0) |
| [#12](https://github.com/Mujshanan/forma-deform/pull/12) | Closed, unmerged | `main` | `9003088254d7301522eef474ff560db2eca81860` | `proposal/native-smart-controller-tool-mcp` | `b096a7219a9af5152a9b1afe7ff138a42836a873` | `—` | — (+0/−0) |
| [#13](https://github.com/Mujshanan/forma-deform/pull/13) | Closed, unmerged | `proposal/native-smart-controller-tool-mcp` | `b096a7219a9af5152a9b1afe7ff138a42836a873` | `audit/anicraft-native-tool-feasibility` | `ec9c8b4738cb396c78bf53829df142b7be9d63cf` | `—` | — (+0/−0) |
| [#14](https://github.com/Mujshanan/forma-deform/pull/14) | Closed, unmerged | `audit/anicraft-native-tool-feasibility` | `ec9c8b4738cb396c78bf53829df142b7be9d63cf` | `core/characterize-consolidate-smart-controller` | `9c658849056f5c97da6b7d6281addccc2b2f78a0` | `—` | — (+0/−0) |
| [#15](https://github.com/Mujshanan/forma-deform/pull/15) | Closed, unmerged | `core/characterize-consolidate-smart-controller` | `9c658849056f5c97da6b7d6281addccc2b2f78a0` | `design/native-controller-properties-and-symbols` | `0688aa439f55d48887659ed74c7d55a5f8f7a19d` | `—` | — (+0/−0) |
| [#16](https://github.com/Mujshanan/forma-deform/pull/16) | Closed, unmerged | `core/characterize-consolidate-smart-controller` | `9c658849056f5c97da6b7d6281addccc2b2f78a0` | `design/native-smart-controller-authoring` | `67c6081caa157aac452479218819d9741317d266` | `—` | — (+0/−0) |
| [#17](https://github.com/Mujshanan/forma-deform/pull/17) | Closed, unmerged | `core/characterize-consolidate-smart-controller` | `9c658849056f5c97da6b7d6281addccc2b2f78a0` | `feat/fd-m1-canonical-mapper` | `4bf71a8b4f645d3d589b2bcbd6708ef21c33fe70` | `—` | — (+0/−0) |
| [#18](https://github.com/Mujshanan/forma-deform/pull/18) | Merged → main | `main` | `4b962368834ba20d0fbba68497336e78dc600914` | `feat/fd-m1-main-integration` | `1264b0e58ec3600fab030d6a64169db514914f38` | `c08ab0fc541ab977d121a41d0e20af150ae52d02` | — (+0/−0) |
| [#19](https://github.com/Mujshanan/forma-deform/pull/19) | Closed, unmerged | `main` | `e182e447a9e6b320507982b98069dc90e5b6e323` | `docs/fdm-v2-architecture-consolidation` | `71df3f65258017c78208268c54fb4209ec8272e7` | `—` | — (+0/−0) |
| [#20](https://github.com/Mujshanan/forma-deform/pull/20) | Closed, unmerged | `docs/fdm-v2-architecture-consolidation` | `71df3f65258017c78208268c54fb4209ec8272e7` | `design/m1-target-geometry-contract` | `9df6bf529d5cefe0bd381ab42e3427915a46b500` | `—` | — (+0/−0) |
| [#21](https://github.com/Mujshanan/forma-deform/pull/21) | Closed, unmerged | `main` | `c08ab0fc541ab977d121a41d0e20af150ae52d02` | `feat/fd-m2-plugin-preview` | `0bded520ba44478e1d5233d3dcb77f33e05f02be` | `—` | — (+0/−0) |
| [#22](https://github.com/Mujshanan/forma-deform/pull/22) | Closed, unmerged | `feat/fd-m2-plugin-preview` | `0bded520ba44478e1d5233d3dcb77f33e05f02be` | `feat/fd-m3-authoring-ui-spec` | `76029b8913535aec3672592bbf7a0e0e4a2345dc` | `—` | — (+0/−0) |
| [#23](https://github.com/Mujshanan/forma-deform/pull/23) | Closed, unmerged | `feat/fd-m3-authoring-ui-spec` | `76029b8913535aec3672592bbf7a0e0e4a2345dc` | `feat/fd-m4-persistent-bindings` | `20dab531787d1ff53e90d23b97bcdd570002fb44` | `—` | — (+0/−0) |
| [#24](https://github.com/Mujshanan/forma-deform/pull/24) | Closed, unmerged | `feat/fd-m4-persistent-bindings` | `20dab531787d1ff53e90d23b97bcdd570002fb44` | `feat/fd-m5-deformation-integration` | `869bb3c859e977174f6c7c42ecdba710fb37fd99` | `—` | — (+0/−0) |
| [#25](https://github.com/Mujshanan/forma-deform/pull/25) | Closed, unmerged | `main` | `e7c7fef4e82345af311370426450146a4795523d` | `feat/m3a-command-authoring` | `f2ad3d521bcf597f128c6149d0037629f889fa06` | `—` | — (+0/−0) |
| [#26](https://github.com/Mujshanan/forma-deform/pull/26) | Closed, unmerged | `main` | `e7c7fef4e82345af311370426450146a4795523d` | `feat/m3b-host-contract` | `6680c5476cc81fbf88a5e2a8cdffe1a7d193b50e` | `—` | — (+0/−0) |
| [#27](https://github.com/Mujshanan/forma-deform/pull/27) | Closed, unmerged | `feat/fd-m5-deformation-integration` | `49b72f72ca9feaac2390b33a20a5e89d4607d069` | `feat/fd-m7-multiaxis-pose-field` | `954d67fe73a34bf161a77f2c7cddf6f6fbb7984e` | `—` | — (+0/−0) |
| [#28](https://github.com/Mujshanan/forma-deform/pull/28) | Closed, unmerged | `feat/fd-m5-deformation-integration` | `b8b9e17ccf50a68b6c5417a2cc52408494cf8c6f` | `feat/fd-m6-host-evaluation-contract` | `0ac2c5bf6847d343d9925dfe904d5375e34f6bd9` | `—` | — (+0/−0) |
| [#29](https://github.com/Mujshanan/forma-deform/pull/29) | Closed, unmerged | `feat/fd-m7-multiaxis-pose-field` | `954d67fe73a34bf161a77f2c7cddf6f6fbb7984e` | `docs/code-reuse-registry` | `7888985a246e325f13a7915bb644b2e6a0ae99f6` | `—` | — (+0/−0) |
| [#30](https://github.com/Mujshanan/forma-deform/pull/30) | Closed, unmerged | `design/m1-target-geometry-contract` | `9df6bf529d5cefe0bd381ab42e3427915a46b500` | `design/m1-evaluation-lifecycle-contracts` | `dc7784f22a527f17cbbe3f95681ed7cea82d2bed` | `—` | — (+0/−0) |
| [#31](https://github.com/Mujshanan/forma-deform/pull/31) | Closed, unmerged | `design/m1-evaluation-lifecycle-contracts` | `dc7784f22a527f17cbbe3f95681ed7cea82d2bed` | `feat/m1-persistence-composition-validation` | `29479aa8c7c2acb8f9b6e522ab80a6838fd2dfea` | `—` | — (+0/−0) |
| [#32](https://github.com/Mujshanan/forma-deform/pull/32) | Closed, unmerged | `feat/m1-persistence-composition-validation` | `29479aa8c7c2acb8f9b6e522ab80a6838fd2dfea` | `feat/m1-3-composition-evaluation-contracts` | `f4d4b0a035526674e8543c6fd7e59b9552950659` | `—` | — (+0/−0) |
| [#33](https://github.com/Mujshanan/forma-deform/pull/33) | Closed, unmerged | `feat/m1-3-composition-evaluation-contracts` | `6b710a637926d5f1077834495d6e60da36484b05` | `feat/fdm-01-1-geometry-contract-integration` | `946425fc2b23a119c63bfdb475fb649f02155287` | `—` | — (+0/−0) |
| [#34](https://github.com/Mujshanan/forma-deform/pull/34) | Closed, unmerged | `feat/fdm-01-1-geometry-contract-integration` | `946425fc2b23a119c63bfdb475fb649f02155287` | `feat/fdm-01-2-operator-contract` | `66a51d08a28bcb0aaf0247a10d54098456c0a346` | `—` | — (+0/−0) |
| [#35](https://github.com/Mujshanan/forma-deform/pull/35) | Closed, unmerged | `feat/fdm-01-2-operator-contract` | `66a51d08a28bcb0aaf0247a10d54098456c0a346` | `feat/fdm-01-3-single-handle-displacement` | `91a383ecc3ec08fa93d0b35fcc49ee5377f668e9` | `—` | — (+0/−0) |
| [#36](https://github.com/Mujshanan/forma-deform/pull/36) | Closed, unmerged | `feat/fdm-01-3-single-handle-displacement` | `91a383ecc3ec08fa93d0b35fcc49ee5377f668e9` | `feat/fdm-01-4-single-operator-evaluation-engine` | `0a1536b0a5396f112452d5cde0fd5bb858b47baa` | `—` | — (+0/−0) |
| [#37](https://github.com/Mujshanan/forma-deform/pull/37) | Closed, unmerged | `feat/fdm-01-4-single-operator-evaluation-engine` | `0a1536b0a5396f112452d5cde0fd5bb858b47baa` | `feat/fdm-02-6-global-cage-embedding-validation` | `92f0324d0fbfaa448bfb3a766c0b7a67c3c4e965` | `—` | — (+0/−0) |
| [#38](https://github.com/Mujshanan/forma-deform/pull/38) | Merged → feature branch (not main) | `quality/cq2-rustfmt-clean` | `ad92dbe6dcf51e822443086bf095e70ae10bc718` | `quality/cq2-d1-controller-validation` | `1db2caaf3ce3c44c801cfaa9af01474796acd7a6` | `612f2ea8522ef9f29c0b18f4792df6d6dab3812e` | — (+0/−0) |
| [#39](https://github.com/Mujshanan/forma-deform/pull/39) | Closed, unmerged | `quality/cq2-rustfmt-clean` | `ad92dbe6dcf51e822443086bf095e70ae10bc718` | `quality/cq2-d2-weighted-composition` | `5e9b8865f71007e7111d72f4c72292bd0bed387f` | `—` | — (+0/−0) |
| [#40](https://github.com/Mujshanan/forma-deform/pull/40) | Closed, unmerged | `feat/fdm-02-6-global-cage-embedding-validation` | `92f0324d0fbfaa448bfb3a766c0b7a67c3c4e965` | `feat/fdm-02-7-cage-binding-system` | `83ef805017fd75f8bc62395b1499dc38589b37fe` | `—` | — (+0/−0) |
| [#41](https://github.com/Mujshanan/forma-deform/pull/41) | Closed, unmerged | `quality/cq2-rustfmt-clean` | `ad92dbe6dcf51e822443086bf095e70ae10bc718` | `quality/cq2-d3-dag-evaluation` | `39772a27751a87610199db000ed242f358a23cbe` | `—` | — (+0/−0) |
| [#42](https://github.com/Mujshanan/forma-deform/pull/42) | Closed, unmerged | `quality/cq2-rustfmt-clean` | `ad92dbe6dcf51e822443086bf095e70ae10bc718` | `quality/native-host-traceability-docs` | `ca8302fdd9e3804b0dc4749b5287602a4c0cc05d` | `—` | — (+0/−0) |
| [#43](https://github.com/Mujshanan/forma-deform/pull/43) | Closed, unmerged | `quality/cq2-rustfmt-clean` | `ad92dbe6dcf51e822443086bf095e70ae10bc718` | `quality/cq2-ci-rustfmt-check` | `4719a592b006869ebd88a5e5c42e634830e410e8` | `—` | — (+0/−0) |
| [#44](https://github.com/Mujshanan/forma-deform/pull/44) | Closed, unmerged | `quality/cq2-ci-rustfmt-check` | `4719a592b006869ebd88a5e5c42e634830e410e8` | `quality/cq2-clippy-report-only` | `05242267fc7357b561d4e29de70d559562c4700d` | `—` | — (+0/−0) |
| [#45](https://github.com/Mujshanan/forma-deform/pull/45) | Closed, unmerged | `quality/cq2-tool-pilot` | `975045cbcd43bd9b1e2195041d06a057d514721d` | `quality/cq2-rustfmt-clean` | `383879a3b05554938e352240f2220874f08bd22e` | `—` | — (+0/−0) |
| [#46](https://github.com/Mujshanan/forma-deform/pull/46) | Closed, unmerged | `feat/fdm-01-4-single-operator-evaluation-engine` | `0a1536b0a5396f112452d5cde0fd5bb858b47baa` | `feat/fdm-01-5-regression-quality-gates` | `f7d1183108a248de2d9560fef1797c682f286ef1` | `—` | — (+0/−0) |
| [#47](https://github.com/Mujshanan/forma-deform/pull/47) | Closed, unmerged | `feat/fdm-02-1-identity-topology-contracts` | `4b5f29cd15e9bb1409b11e6eb49a5e645953a314` | `test/fdm-03-v1-integration-regressions` | `5348119f54be329f6fa022d6591b049314c803de` | `—` | — (+0/−0) |
| [#48](https://github.com/Mujshanan/forma-deform/pull/48) | Merged → feature branch (not main) | `test/fdm-03-v1-integration-regressions` | `736764d9ca6aba371bd52b0acd1bc6bb979ae26e` | `feat/fdm-04-binding-evaluation-engine` | `271200f938c6836964d5df29cbc223cb8d948e14` | `4ae97ebfbfbafced0dfc285a090e1f3c3d8c9a81` | — (+0/−0) |
| [#49](https://github.com/Mujshanan/forma-deform/pull/49) | Closed, unmerged | `feat/fdm-02-1-identity-topology-contracts` | `70ec1b3a55d602de93a5b9424fcd8dad99b35807` | `docs/leg-01-4-adapter-retirement-status` | `f25d1e6805b852a5dc0f63f47e5d78828b1c4b7e` | `—` | — (+0/−0) |
| [#50](https://github.com/Mujshanan/forma-deform/pull/50) | Merged → feature branch (not main) | `test/fdm-03-v1-integration-regressions` | `4ae97ebfbfbafced0dfc285a090e1f3c3d8c9a81` | `feat/fdm-04-2-adaptive-curve-sample-evaluation` | `10635118df9ee6b7b704b8f4b01b9cae4a44066e` | `441b27e7c03f44074ffe9fe68720dbcc865d3765` | — (+0/−0) |
| [#51](https://github.com/Mujshanan/forma-deform/pull/51) | Merged → feature branch (not main) | `feat/fdm-04-2-adaptive-curve-sample-evaluation` | `10635118df9ee6b7b704b8f4b01b9cae4a44066e` | `test/fdm-04-3-numerical-regressions` | `a9c49b23f17da0be642730db84ae1860d66e5cd3` | `c49db736e214fdf85a3e6ea3567d72b51c0bb250` | — (+0/−0) |
| [#52](https://github.com/Mujshanan/forma-deform/pull/52) | Merged → feature branch (not main) | `docs/leg-01-4-adapter-retirement-status` | `8651212643a5c27f2767efe31575d7972481a126` | `chore/retire-bun-demo-and-fix-workspace-tests` | `581228a7373eb87f841fa2b26505b6b997ad2afd` | `f25d1e6805b852a5dc0f63f47e5d78828b1c4b7e` | — (+0/−0) |
| [#53](https://github.com/Mujshanan/forma-deform/pull/53) | Closed, unmerged | `test/fdm-04-3-numerical-regressions` | `a9c49b23f17da0be642730db84ae1860d66e5cd3` | `perf/fdm-04-4-performance-quality-gates` | `af752c46778b42c492b2fdf1dd9943145daeb3aa` | `—` | — (+0/−0) |
| [#54](https://github.com/Mujshanan/forma-deform/pull/54) | Merged → feature branch (not main) | `test/fdm-04-3-numerical-regressions` | `a9c49b23f17da0be642730db84ae1860d66e5cd3` | `perf/fdm-04-4-sampling-quality-gates` | `7aba19b31dbd030d7cd32b83832aed5ee32002cc` | `4e684558f966d134677df89afa88777f25fd46b3` | — (+0/−0) |
| [#55](https://github.com/Mujshanan/forma-deform/pull/55) | Closed, unmerged | `test/fdm-03-v1-integration-regressions` | `441b27e7c03f44074ffe9fe68720dbcc865d3765` | `fix/fdm-04-5-integration-hardening` | `3082f73eb01dca6130057232aebb00e275a74b2f` | `—` | — (+0/−0) |
| [#56](https://github.com/Mujshanan/forma-deform/pull/56) | Merged → feature branch (not main) | `test/fdm-03-v1-integration-regressions` | `fbf5acea7a7fdc14b23d0ea761a98be7b1de7d7f` | `integration/fdm-04-5-clean-lineage` | `23f9bfdae99f3f8fa0ecf4df4877d58c4b95f3ff` | `484fc2127340568b555db2ff7364da1a7aa7bb21` | — (+0/−0) |
| [#57](https://github.com/Mujshanan/forma-deform/pull/57) | Closed, unmerged | `test/fdm-03-v1-integration-regressions` | `484fc2127340568b555db2ff7364da1a7aa7bb21` | `feat/fdm-05-deformation-output-integration` | `edb524bc46248b755fa2b0646aaa9cac72fcc479` | `—` | — (+0/−0) |
| [#58](https://github.com/Mujshanan/forma-deform/pull/58) | Closed, unmerged | `feat/fdm-05-deformation-output-integration` | `edb524bc46248b755fa2b0646aaa9cac72fcc479` | `feat/fdm-05-3-structured-derived-assembly` | `902514675f2f9cf1db760213af3e0e880dd3715c` | `—` | — (+0/−0) |
| [#59](https://github.com/Mujshanan/forma-deform/pull/59) | Closed, unmerged | `feat/fdm-05-3-structured-derived-assembly` | `902514675f2f9cf1db760213af3e0e880dd3715c` | `test/fdm-05-3-assembly-constructor-regressions` | `ea5bf9a0cc85cc11bb7a9148d449d547ce40a578` | `—` | — (+0/−0) |
| [#60](https://github.com/Mujshanan/forma-deform/pull/60) | Closed, unmerged | `test/fdm-05-3-assembly-constructor-regressions` | `ea5bf9a0cc85cc11bb7a9148d449d547ce40a578` | `hardening/fdm-05-4-output-contract` | `e6a94e864ef12f377694c0c307c13e45b1f7dd53` | `—` | — (+0/−0) |
| [#61](https://github.com/Mujshanan/forma-deform/pull/61) | Closed, unmerged | `test/fdm-03-v1-integration-regressions` | `5348119f54be329f6fa022d6591b049314c803de` | `chore/pr-47-1-clippy-audit` | `61c8320c484b61f0ca6f8a0b0088c3e1664b978b` | `—` | — (+0/−0) |
| [#62](https://github.com/Mujshanan/forma-deform/pull/62) | Closed, unmerged | `chore/pr-47-1-clippy-audit` | `61c8320c484b61f0ca6f8a0b0088c3e1664b978b` | `chore/pr-47-2-workspace-formatting-ownership-audit` | `375fa15f2b9107d2f50b318547c2bad634b4f536` | `—` | — (+0/−0) |
| [#63](https://github.com/Mujshanan/forma-deform/pull/63) | Closed, unmerged | `chore/pr-47-2-workspace-formatting-ownership-audit` | `375fa15f2b9107d2f50b318547c2bad634b4f536` | `fix/pr-47-3-path2d-persistence-cas` | `f2df76d807a7375a4eb96c20432f4ad2ed0b30b8` | `—` | — (+0/−0) |
| [#64](https://github.com/Mujshanan/forma-deform/pull/64) | Closed, unmerged | `hardening/fdm-05-4-output-contract` | `e6a94e864ef12f377694c0c307c13e45b1f7dd53` | `design/fdm-06-2-native-integration-contract` | `9fb3659f181925eda7dd317e775040359f696a98` | `—` | — (+0/−0) |
| [#65](https://github.com/Mujshanan/forma-deform/pull/65) | Closed, unmerged | `design/fdm-06-2-native-integration-contract` | `9fb3659f181925eda7dd317e775040359f696a98` | `plan/fdm-06-3-host-contract-conformance` | `18ca0354911ab3447cba98e7f42dc8d3a4329937` | `—` | — (+0/−0) |
| [#66](https://github.com/Mujshanan/forma-deform/pull/66) | Closed, unmerged | `plan/fdm-06-3-host-contract-conformance` | `18ca0354911ab3447cba98e7f42dc8d3a4329937` | `design/fdm-06-4-host-extension-identity` | `4942b81a6d0daa11f5d9adced43b3ad9286eabdf` | `—` | — (+0/−0) |
| [#67](https://github.com/Mujshanan/forma-deform/pull/67) | Closed, unmerged | `design/fdm-06-4-host-extension-identity` | `4942b81a6d0daa11f5d9adced43b3ad9286eabdf` | `audit/fdm-06-5-target-identity-revisions` | `2494aaaf5e4d55ecb19c13c31ea8b22c183e29e9` | `—` | — (+0/−0) |
| [#68](https://github.com/Mujshanan/forma-deform/pull/68) | Closed, unmerged | `chore/pr-47-2-workspace-formatting-ownership-audit` | `375fa15f2b9107d2f50b318547c2bad634b4f536` | `chore/fdm-fmt-01-workspace-formatting-remediation` | `813ef8807dbfecb147e3d3f4534777704c1b8cb8` | `—` | — (+0/−0) |
| [#69](https://github.com/Mujshanan/forma-deform/pull/69) | Closed, unmerged | `audit/fdm-06-5-target-identity-revisions` | `a67a2391365ffbe3491db1832e64eb304c4e2987` | `design/fdm-06-6-mutation-invalidation-contract` | `62621ec548da0ec3fd859949304338d8f01550be` | `—` | — (+0/−0) |
| [#70](https://github.com/Mujshanan/forma-deform/pull/70) | Closed, unmerged | `design/fdm-06-6-mutation-invalidation-contract` | `62621ec548da0ec3fd859949304338d8f01550be` | `review/fdm-06-7-mutation-boundary-readiness` | `88f648e13d3c7629d773e2d5a8bea504242f2bc2` | `—` | — (+0/−0) |
| [#71](https://github.com/Mujshanan/forma-deform/pull/71) | Closed, unmerged | `design/fdm-06-2-native-integration-contract` | `9fb3659f181925eda7dd317e775040359f696a98` | `plan/fdm-06-3-host-conformance` | `c526541ed864975736f50c85017f2f37c927fb86` | `—` | — (+0/−0) |
| [#72](https://github.com/Mujshanan/forma-deform/pull/72) | Closed, unmerged | `review/fdm-06-7-mutation-boundary-readiness` | `88f648e13d3c7629d773e2d5a8bea504242f2bc2` | `spec/fdm-06-8-mutation-state-ownership` | `cce907a1c961b5676454b588ee89414382fc90fa` | `—` | — (+0/−0) |
| [#73](https://github.com/Mujshanan/forma-deform/pull/73) | Closed, unmerged | `spec/fdm-06-8-mutation-state-ownership` | `cce907a1c961b5676454b588ee89414382fc90fa` | `plan/fdm-06-9-host-transaction-api` | `da15372367a77af3f8a209b36c94ced4743ef110` | `—` | — (+0/−0) |
| [#74](https://github.com/Mujshanan/forma-deform/pull/74) | Closed, unmerged | `audit/fdm-06-5-target-identity-revisions` | `2494aaaf5e4d55ecb19c13c31ea8b22c183e29e9` | `design/fdm-06-6-host-mutation-invalidation` | `c5302da17bdeb84f9d578613732e1b5a59808164` | `—` | — (+0/−0) |
| [#75](https://github.com/Mujshanan/forma-deform/pull/75) | Closed, unmerged | `plan/fdm-06-9-host-transaction-api` | `da15372367a77af3f8a209b36c94ced4743ef110` | `audit/fdm-06-10-s0-mutation-inventory` | `af162e492e87fc1864641f6f18e54f544b1016e2` | `—` | — (+0/−0) |
| [#76](https://github.com/Mujshanan/forma-deform/pull/76) | Closed, unmerged | `audit/fdm-06-10-s0-mutation-inventory` | `af162e492e87fc1864641f6f18e54f544b1016e2` | `audit/fdm-06-11-s0-registry-ownership-closure` | `d5c752c8a1e5d8e05f4976605cdb5a62e1d7a3f3` | `—` | — (+0/−0) |
| [#77](https://github.com/Mujshanan/forma-deform/pull/77) | Closed, unmerged | `audit/fdm-06-10-s0-mutation-inventory` | `af162e492e87fc1864641f6f18e54f544b1016e2` | `audit/fdm-06-11-s0-registry-ownership` | `5ec4931b70efc1b6082f7e4516fe462017df5d4b` | `—` | — (+0/−0) |
| [#78](https://github.com/Mujshanan/forma-deform/pull/78) | Closed, unmerged | `audit/fdm-06-11-s0-registry-ownership-closure` | `d5c752c8a1e5d8e05f4976605cdb5a62e1d7a3f3` | `plan/fdm-06-12-mutation-boundary-implementation` | `67dde92057816aba83f49ca8ae00cab6bc882e1e` | `—` | — (+0/−0) |
| [#79](https://github.com/Mujshanan/forma-deform/pull/79) | Closed, unmerged | `audit/fdm-06-11-s0-registry-ownership` | `5ec4931b70efc1b6082f7e4516fe462017df5d4b` | `audit/fdm-06-12-workspace-consumer-baseline` | `5d229d247e03905c9b438c977a13242c0a73682b` | `—` | — (+0/−0) |
| [#80](https://github.com/Mujshanan/forma-deform/pull/80) | Closed, unmerged | `audit/fdm-06-12-workspace-consumer-baseline` | `5d229d247e03905c9b438c977a13242c0a73682b` | `audit/fdm-06-13-s0-closure-review-evidence` | `451309f480dfb178ec08a4172c48a339744b7621` | `—` | — (+0/−0) |
| [#81](https://github.com/Mujshanan/forma-deform/pull/81) | Closed, unmerged | `main` | `e182e447a9e6b320507982b98069dc90e5b6e323` | `docs/update-project-status-2026-11-23` | `1fddd2bd00dc638e3994e9e91993ae22fdb6dd58` | `—` | — (+0/−0) |
| [#82](https://github.com/Mujshanan/forma-deform/pull/82) | Closed, unmerged | `main` | `e182e447a9e6b320507982b98069dc90e5b6e323` | `chore/repository-consolidation-2026-11-23` | `3193ebbc5571e096883202ffa904734950cede2e` | `—` | — (+0/−0) |
| [#83](https://github.com/Mujshanan/forma-deform/pull/83) | OPEN | `main` | `e182e447a9e6b320507982b98069dc90e5b6e323` | `consolidation/final-pr-register-2026-11-23` | `9e5e355676bc4546f651090cce3f2e0bce8f1214` | `—` | — (+0/−0) |
| [#84](https://github.com/Mujshanan/forma-deform/pull/84) | Merged → main | `main` | `e182e447a9e6b320507982b98069dc90e5b6e323` | `docs/native-ui-m1-status-2026-11-23` | `d974efb222aa9303b638feca8a47189df15e58ec` | `d936b3b3b41e788161fe84acd23421ad51c2c73a` | — (+0/−0) |
| [#85](https://github.com/Mujshanan/forma-deform/pull/85) | Merged → main | `main` | `d936b3b3b41e788161fe84acd23421ad51c2c73a` | `docs/native-ui-handle-drag-status-2026-11-23` | `02573fece7d4b5690bab5c1b72336224c699c9a5` | `7ae59f4f51da19b0e2160deed966ff5da8870903` | — (+0/−0) |

## 3. Current branch-level recovery candidates

These are branch-to-`main` comparisons, not proposed merges. The current `main` SHA above is the comparison target.

| Branch | Observed HEAD | Comparison to `main` | CI evidence | Recovery assessment |
|---|---|---|---|---|
| `test/fdm-03-v1-integration-regressions` | `5348119f54be329f6fa022d6591b049314c803de` | 313 commits ahead, 2 behind; merge base `e182e447a9e6b320507982b98069dc90e5b6e323` | CI run [45164729008](https://github.com/Mujshanan/forma-deform/actions/runs/45164729008): success | Main FDM-01–04.5 recovery candidate. Includes feature-branch merges #48/#50/#51/#54/#56. Not integrated into main. |
| `fix/fdm-fmt-01-2-ci-runner` | `813ef8807dbfecb147e3d3f4534777704c1b8cb8` | 327 commits ahead, 2 behind; same merge base `e182e447a9e6b320507982b98069dc90e5b6e323`; 14 commits ahead of the FDM-03 branch | CI run [45171640134](https://github.com/Mujshanan/forma-deform/actions/runs/45171640134): failure at `cargo fmt --all -- --check`; Clippy, workspace tests, benchmark and three WASM builds were skipped | Separate formatting-maintenance candidate. Not ready to integrate until the formatting diff is completed and exact scoped checks pass. |
| `audit/fdm-06-13-s0-closure-review-evidence` | `451309f480dfb178ec08a4172c48a339744b7621` | 330 commits ahead, 2 behind; same merge base `e182e447a9e6b320507982b98069dc90e5b6e323` | CI run [45207610660](https://github.com/Mujshanan/forma-deform/actions/runs/45207610660): success | Contains FDM-05 output code and FDM-06.13–06.17 S0 evidence. Audit/documentation branch; S0 remains OPEN. CI is Forma Deform CI, not AniCraft host CI. |
| `feat/fd-m1-main-integration` | `1264b0e58ec3600fab030d6a64169db514914f38` | 10 commits ahead, 7 behind; merge base `4b962368834ba20d0fbba68497336e78dc600914` | CI run [44997799376](https://github.com/Mujshanan/forma-deform/actions/runs/44997799376): success | Smart Controller mapper/adapter candidate after PR #18. Nine-file delta includes canonical mapper, composition/mapping, mapper tests and plug-in adapter/preview. Not integrated into current main. |
| `consolidation/final-pr-register-2026-11-23` | `9e5e355676bc4546f651090cce3f2e0bce8f1214` | 2 commits ahead, 2 behind; merge base `e182e447a9e6b320507982b98069dc90e5b6e323` | CI run [45208172135](https://github.com/Mujshanan/forma-deform/actions/runs/45208172135): success | Documentation-only PR #83. Its diff against current main is the single new register file; it has not been merged. |

### 3.1 Branch overlap and divergence

- The FDM-03 branch and FDM-FMT branch share the same merge base, but the FDM-FMT branch is 14 commits ahead of FDM-03. Its diff against FDM-03 touches CI, several `form-deform-core` files and formatting/audit documentation.
- The S0 evidence branch and FDM-FMT branch diverge from merge base `484fc2127340568b555db2ff7364da1a7aa7bb21`: the S0 branch is 76 commits ahead of the FDM-FMT branch, while the FDM-FMT branch is 73 commits ahead of the S0 branch. Their overlapping core files make a blind merge or cherry-pick unsafe.
- PR #47's base branch `feat/fd-m02-1-identity-topology-contracts` is absent from the current branch inventory. Its head branch remains present, so the PR head history is still recoverable.
- All PR head branch names #1–#85 were present in the current branch inventory at this snapshot. This does not imply that all base branches still exist or that their contents are integrated.
- PRs #84 and #85 are documentation-only merges to `main`; they record partial native M1 UI progress and do not claim persistent bindings, mapping/evaluation, target application, undo integration or live playback.

## 4. S0 evidence status

PR #80 remains closed and unmerged; its head branch is preserved. Its submitted evidence explicitly says **S0 NOT CLOSED**. The latest FDM-06.17 report remains partial/in progress:
- The original 36-row inline-callback manifest is not yet exactly reconciled.
- The 131 false-flag callback paths still lack complete transitive closure; the remaining 264 commands do not have a complete graph.
- The 762 candidate consumer call-site lines across 94 files are a lexical inventory, not compiler-backed proof.
- Host tests, Clippy, cargo-deny and macOS/Windows CI on the pinned AniCraft baseline remain outstanding.

The successful CI runs listed above are Forma Deform repository runs. They do not satisfy the AniCraft host CI requirement and do not close S0.

## 5. Recovery rules and current disposition

1. Do not merge old PRs wholesale. Use their exact PR diffs and commit ancestry to recover only unique, still-valid work.
2. Do not infer original review-time head SHAs from a reused branch's current tip. Inspect the PR commit list and merge commit before selecting commits.
3. Keep FDM core, Smart Controller, FDM-05, S0 evidence and formatting as separately reviewed workstreams until overlap is reconciled.
4. Keep formatting-only changes separate from behavior changes.
5. S0 remains OPEN. No native mutation-boundary implementation, runtime integration, apply/bake, undo change or cross-engine composition is authorized by this audit.
6. No source commits were cherry-picked, rebased, merged or moved by this matrix creation.

**Result:** the repository history is recoverable, but it is not one linear branch. The next safe action is candidate-specific commit and semantic diff review—not a mass merge.
