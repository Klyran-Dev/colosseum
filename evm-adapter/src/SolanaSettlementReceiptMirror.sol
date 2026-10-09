// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.37;

/// @notice Non-custodial EVM projection of finalized Solana settlement evidence.
/// @dev This contract deliberately has no token interfaces, payable methods, or
/// external calls. It cannot authorize or move funds on Solana or on EVM.
contract SolanaSettlementReceiptMirror {
    struct Receipt {
        bytes32 sourceDigest;
        bytes32 merchant;
        bytes32 mint;
        uint64 slot;
        uint64 grossAtomic;
        uint64 taxAtomic;
        uint64 netAtomic;
        uint64 recordedAt;
        bool exists;
    }

    error Unauthorized();
    error Paused();
    error InvalidReceipt();
    error ConflictingReceipt();
    error ReceiptNotFound();
    error ZeroAddress();

    event ReceiptMirrored(
        bytes32 indexed receiptId,
        bytes32 indexed sourceDigest,
        bytes32 merchant,
        bytes32 mint,
        uint64 slot,
        uint64 grossAtomic,
        uint64 taxAtomic,
        uint64 netAtomic
    );
    event IssuerChanged(address indexed previousIssuer, address indexed nextIssuer);
    event GovernanceTransferStarted(address indexed previousGovernance, address indexed pendingGovernance);
    event GovernanceTransferred(address indexed previousGovernance, address indexed nextGovernance);
    event PausedStateChanged(bool paused);

    address public governance;
    address public pendingGovernance;
    address public issuer;
    bytes32 public immutable sourceDomain;
    bool public paused;

    mapping(bytes32 => Receipt) private receipts;

    modifier onlyGovernance() {
        if (msg.sender != governance) revert Unauthorized();
        _;
    }

    modifier onlyIssuer() {
        if (msg.sender != issuer) revert Unauthorized();
        _;
    }

    constructor(address initialGovernance, address initialIssuer, bytes32 initialSourceDomain) {
        if (initialGovernance == address(0) || initialIssuer == address(0) || initialSourceDomain == bytes32(0)) {
            revert InvalidReceipt();
        }
        governance = initialGovernance;
        issuer = initialIssuer;
        sourceDomain = initialSourceDomain;
    }

    /// @notice Store one finalized, server-verified Solana receipt projection.
    /// @dev An exact replay is a no-op. A changed replay is rejected.
    function recordReceipt(
        bytes32 receiptId,
        bytes32 sourceDigest,
        bytes32 merchant,
        bytes32 mint,
        uint64 slot,
        uint64 grossAtomic,
        uint64 taxAtomic,
        uint64 netAtomic
    ) external onlyIssuer {
        if (paused) revert Paused();
        if (
            receiptId == bytes32(0) || sourceDigest == bytes32(0) || merchant == bytes32(0) || mint == bytes32(0)
                || slot == 0 || grossAtomic == 0 || taxAtomic > grossAtomic || netAtomic != grossAtomic - taxAtomic
        ) {
            revert InvalidReceipt();
        }

        Receipt storage current = receipts[receiptId];
        if (current.exists) {
            if (
                current.sourceDigest == sourceDigest && current.merchant == merchant && current.mint == mint
                    && current.slot == slot && current.grossAtomic == grossAtomic && current.taxAtomic == taxAtomic
                    && current.netAtomic == netAtomic
            ) {
                return;
            }
            revert ConflictingReceipt();
        }

        current.sourceDigest = sourceDigest;
        current.merchant = merchant;
        current.mint = mint;
        current.slot = slot;
        current.grossAtomic = grossAtomic;
        current.taxAtomic = taxAtomic;
        current.netAtomic = netAtomic;
        current.recordedAt = uint64(block.timestamp);
        current.exists = true;
        emit ReceiptMirrored(
            receiptId,
            sourceDigest,
            merchant,
            mint,
            slot,
            grossAtomic,
            taxAtomic,
            netAtomic
        );
    }

    function getReceipt(bytes32 receiptId)
        external
        view
        returns (
            bytes32 sourceDigest,
            bytes32 merchant,
            bytes32 mint,
            uint64 slot,
            uint64 grossAtomic,
            uint64 taxAtomic,
            uint64 netAtomic,
            uint64 recordedAt
        )
    {
        Receipt storage receipt = receipts[receiptId];
        if (!receipt.exists) revert ReceiptNotFound();
        return (
            receipt.sourceDigest,
            receipt.merchant,
            receipt.mint,
            receipt.slot,
            receipt.grossAtomic,
            receipt.taxAtomic,
            receipt.netAtomic,
            receipt.recordedAt
        );
    }

    function receiptExists(bytes32 receiptId) external view returns (bool) {
        return receipts[receiptId].exists;
    }

    function setIssuer(address nextIssuer) external onlyGovernance {
        if (nextIssuer == address(0)) revert ZeroAddress();
        address previousIssuer = issuer;
        issuer = nextIssuer;
        emit IssuerChanged(previousIssuer, nextIssuer);
    }

    function setPaused(bool nextPaused) external onlyGovernance {
        paused = nextPaused;
        emit PausedStateChanged(nextPaused);
    }

    function transferGovernance(address nextGovernance) external onlyGovernance {
        if (nextGovernance == address(0)) revert ZeroAddress();
        pendingGovernance = nextGovernance;
        emit GovernanceTransferStarted(governance, nextGovernance);
    }

    function acceptGovernance() external {
        if (msg.sender != pendingGovernance) revert Unauthorized();
        address previousGovernance = governance;
        governance = msg.sender;
        pendingGovernance = address(0);
        emit GovernanceTransferred(previousGovernance, msg.sender);
    }
}
