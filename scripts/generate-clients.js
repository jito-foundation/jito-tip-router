const kinobi = require("kinobi");
const anchorIdl = require("@exo-tech-xyz/nodes-from-anchor");
const path = require("path");
const renderers = require('@exo-tech-xyz/renderers');

// Paths.
const projectRoot = path.join(__dirname, "..");

const idlDir = path.join(projectRoot, "idl");

const rustClientsDir = path.join(__dirname, "..", "clients", "rust");
const jsClientsDir = path.join(__dirname, "..", "clients", "js");

// Generate the weight table client in Rust and JavaScript.
const rustWeightTableClientDir = path.join(rustClientsDir, "jito_tip_router");
const jsWeightTableClientDir = path.join(jsClientsDir, "jito_tip_router");
const weightTableRootNode = anchorIdl.rootNodeFromAnchor(require(path.join(idlDir, "jito_tip_router.json")));
const weightTableKinobi = kinobi.createFromRoot(weightTableRootNode);
weightTableKinobi.update(kinobi.bottomUpTransformerVisitor([
    {
        // PodU128 -> u128
        select: (node) => {
            return (
                kinobi.isNode(node, "structFieldTypeNode") &&
                node.type.name === "podU128"
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "structFieldTypeNode");
            return {
                ...node,
                type: kinobi.numberTypeNode("u128"),
            };
        },
    },
    {
        // PodU64 -> u64
        select: (node) => {
            return (
                kinobi.isNode(node, "structFieldTypeNode") &&
                node.type.name === "podU64"
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "structFieldTypeNode");
            return {
                ...node,
                type: kinobi.numberTypeNode("u64"),
            };
        },
    },
    {
        // PodU32 -> u32
        select: (node) => {
            return (
                kinobi.isNode(node, "structFieldTypeNode") &&
                node.type.name === "podU32"
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "structFieldTypeNode");
            return {
                ...node,
                type: kinobi.numberTypeNode("u32"),
            };
        },
    },
    {
        // PodU16 -> u16
        select: (node) => {
            return (
                kinobi.isNode(node, "structFieldTypeNode") &&
                node.type.name === "podU16"
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "structFieldTypeNode");
            return {
                ...node,
                type: kinobi.numberTypeNode("u16"),
            };
        },
    },
    {
        // PodBool -> bool
        select: (node) => {
            return (
                kinobi.isNode(node, "structFieldTypeNode") &&
                node.type.name === "podBool"
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "structFieldTypeNode");
            return {
                ...node,
                type: kinobi.numberTypeNode("bool"),
            };
        },
    },
    // add 8 byte discriminator to accountNode
    {
        select: (node) => {
            return (
                kinobi.isNode(node, "accountNode")
            );
        },
        transform: (node) => {
            kinobi.assertIsNode(node, "accountNode");

            return {
                ...node,
                data: {
                    ...node.data,
                    fields: [
                        kinobi.structFieldTypeNode({ name: 'discriminator', type: kinobi.numberTypeNode('u64') }),
                        ...node.data.fields
                    ]
                }
            };
        },
    },
]));
weightTableKinobi.accept(renderers.renderRustVisitor(path.join(rustWeightTableClientDir, "src", "generated"), {
    formatCode: true,
    crateFolder: rustWeightTableClientDir,
    deleteFolderBeforeRendering: true,
    toolchain: "+nightly-2024-07-25"
}));
weightTableKinobi.accept(renderers.renderJavaScriptVisitor(path.join(jsWeightTableClientDir), {}));
