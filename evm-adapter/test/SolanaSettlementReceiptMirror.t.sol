// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.37;

import {SolanaSettlementReceiptMirror} from "../src/SolanaSettlementReceiptMirror.sol";

interface Vm {
    function prank(address sender) external;
    function expectRevert(bytes4 selector) external;
}

contract SolanaSettlementReceiptMirrorTest {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    address private constant GOVERNANCE = address(0x1001);
    address private constant ISSUER = address(0x1002);
    address private constant NEXT_GOVERNANCE = address(0x1003);
    address private constant NEXT_ISSUER = address(0x1004);

    bytes32 private constant SOURCE_DOMAIN = keccak256("solana-mainnet-beta");
    bytes32 private constant RECEIPT_ID = keccak256("receipt-1");
    bytes32 private constant SOURCE_DIGEST = keccak256("solana-signature");
    bytes32 private constant MERCHANT = keccak256("merchant");
    bytes32 private constant MINT = keccak256("mint");

    SolanaSettlementReceiptMirror private mirror;

    function setUp() public {
        mirror = new SolanaSettlementReceiptMirror(GOVERNANCE, ISSUER, SOURCE_DOMAIN);
    }

    function testRecordsConservingReceiptAndExposesSourceDomain() public {
        _record(100, 25, 75);

        (
            bytes32 sourceDigest,
            bytes32 merchant,
            bytes32 mint,
            uint64 slot,
            uint64 grossAtomic,
            uint64 taxAtomic,
            uint64 netAtomic,
            uint64 recordedAt
        ) = mirror.getReceipt(RECEIPT_ID);

        require(sourceDigest == SOURCE_DIGEST, "source digest");
        require(merchant == MERCHANT, "merchant");
        require(mint == MINT, "mint");
        require(slot == 77, "slot");
        require(grossAtomic == 100 && taxAtomic == 25 && netAtomic == 75, "amounts");
        require(recordedAt > 0, "recorded time");
        require(mirror.sourceDomain() == SOURCE_DOMAIN, "source domain");
    }

    function testIdenticalReplayIsIdempotent() public {
        _record(100, 25, 75);
        _record(100, 25, 75);
        require(mirror.receiptExists(RECEIPT_ID), "receipt missing");
    }

    function testConflictingReplayFailsClosed() public {
        _record(100, 25, 75);
        vm.expectRevert(SolanaSettlementReceiptMirror.ConflictingReceipt.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 24, 76);
    }

    function testOnlyIssuerCanRecord() public {
        vm.expectRevert(SolanaSettlementReceiptMirror.Unauthorized.selector);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 25, 75);
    }

    function testRejectsInvalidConservationAndIdentity() public {
        vm.expectRevert(SolanaSettlementReceiptMirror.InvalidReceipt.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(bytes32(0), SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 25, 75);

        vm.expectRevert(SolanaSettlementReceiptMirror.InvalidReceipt.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 26, 75);
    }

    function testPauseAndIssuerRotationAreGoverned() public {
        vm.prank(GOVERNANCE);
        mirror.setPaused(true);
        vm.expectRevert(SolanaSettlementReceiptMirror.Paused.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 25, 75);

        vm.prank(GOVERNANCE);
        mirror.setPaused(false);
        vm.prank(GOVERNANCE);
        mirror.setIssuer(NEXT_ISSUER);
        vm.expectRevert(SolanaSettlementReceiptMirror.Unauthorized.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 25, 75);
        vm.prank(NEXT_ISSUER);
        mirror.recordReceipt(RECEIPT_ID, SOURCE_DIGEST, MERCHANT, MINT, 77, 100, 25, 75);
    }

    function testGovernanceTransferIsTwoStep() public {
        vm.prank(GOVERNANCE);
        mirror.transferGovernance(NEXT_GOVERNANCE);
        require(mirror.pendingGovernance() == NEXT_GOVERNANCE, "pending governance");
        vm.expectRevert(SolanaSettlementReceiptMirror.Unauthorized.selector);
        mirror.acceptGovernance();
        vm.prank(NEXT_GOVERNANCE);
        mirror.acceptGovernance();
        require(mirror.governance() == NEXT_GOVERNANCE, "governance not accepted");
    }

    function testAllowsMaximumConservingUint64Values() public {
        uint64 maximum = type(uint64).max;
        vm.prank(ISSUER);
        mirror.recordReceipt(
            RECEIPT_ID,
            SOURCE_DIGEST,
            MERCHANT,
            MINT,
            type(uint64).max,
            maximum,
            0,
            maximum
        );
        require(mirror.receiptExists(RECEIPT_ID), "maximum receipt missing");
    }

    function testFuzzConservingAtomicAmounts(uint64 grossAtomic, uint64 taxAtomic) public {
        if (grossAtomic == 0 || taxAtomic > grossAtomic) return;

        _record(grossAtomic, taxAtomic, grossAtomic - taxAtomic);
        (, , , , uint64 storedGross, uint64 storedTax, uint64 storedNet, ) = mirror.getReceipt(RECEIPT_ID);
        require(storedGross == grossAtomic, "gross changed");
        require(storedTax == taxAtomic, "tax changed");
        require(storedNet == grossAtomic - taxAtomic, "net is not conserving");
    }

    function testFuzzRejectsNonConservingAtomicAmounts(uint64 grossAtomic, uint64 taxAtomic) public {
        if (grossAtomic == 0 || taxAtomic > grossAtomic) return;

        uint64 conservingNet = grossAtomic - taxAtomic;
        uint64 nonConservingNet = conservingNet == type(uint64).max ? 0 : conservingNet + 1;
        vm.expectRevert(SolanaSettlementReceiptMirror.InvalidReceipt.selector);
        vm.prank(ISSUER);
        mirror.recordReceipt(
            RECEIPT_ID,
            SOURCE_DIGEST,
            MERCHANT,
            MINT,
            77,
            grossAtomic,
            taxAtomic,
            nonConservingNet
        );
    }

    function _record(uint64 grossAtomic, uint64 taxAtomic, uint64 netAtomic) private {
        vm.prank(ISSUER);
        mirror.recordReceipt(
            RECEIPT_ID,
            SOURCE_DIGEST,
            MERCHANT,
            MINT,
            77,
            grossAtomic,
            taxAtomic,
            netAtomic
        );
    }
}
