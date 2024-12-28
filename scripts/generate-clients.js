const codama = require("codama");
const anchorIdl = require("@codama/nodes-from-anchor");
const path = require("path");
const renderers = require('@codama/renderers');

// Paths.
const projectRoot = path.join(__dirname, "..");

const idlDir = path.join(projectRoot, "idl");

const rustClientsDir = path.join(__dirname, "..", "clients", "rust");
const jsClientsDir = path.join(__dirname, "..", "clients", "js");

// Generate the weight table client in Rust and JavaScript.
const rustWeightTableClientDir = path.join(rustClientsDir, "jito_tip_router");
const jsWeightTableClientDir = path.join(jsClientsDir, "jito_tip_router");
const weightTableRootNode = anchorIdl.rootNodeFromAnchor(require(path.join(idlDir, "jito_tip_router.json")));
const weightTableKinobi = codama.createFromRoot(weightTableRootNode);
weightTableKinobi.update(codama.bottomUpTransformerVisitor([
    {
        // PodU128 -> u128
        select: (nodes) => {
            for (let i = 0; i < nodes.length; i++) {
                if (codama.isNode(nodes[i], "structFieldTypeNode") && nodes[i].type.name === "podU128") {
                    return true;
                }
            }

            return false;
        },
        transform: (node) => {
            codama.assertIsNode(node, ["structFieldTypeNode", "definedTypeLinkNode"]);
            return {
                ...node,
                type: codama.numberTypeNode("u128"),
            };
        },
    },
    {
        // PodU64 -> u64
        select: (nodes) => {
            for (let i = 0; i < nodes.length; i++) {
                if (codama.isNode(nodes[i], "structFieldTypeNode") && nodes[i].type.name === "podU64") {
                    return true;
                }
            }

            return false;
        },
        transform: (node) => {
            codama.assertIsNode(node, ["structFieldTypeNode", "definedTypeLinkNode"]);
            return {
                ...node,
                type: codama.numberTypeNode("u64"),
            };
        },
    },
    {
        // PodU32 -> u32
        select: (nodes) => {
            for (let i = 0; i < nodes.length; i++) {
                if (codama.isNode(nodes[i], "structFieldTypeNode") && nodes[i].type.name === "podU32") {
                    return true;
                }
            }

            return false;
        },
        transform: (node) => {
            codama.assertIsNode(node, ["structFieldTypeNode", "definedTypeLinkNode"]);
            return {
                ...node,
                type: codama.numberTypeNode("u32"),
            };
        },
    },
    {
        // PodU16 -> u16
        select: (nodes) => {
            for (let i = 0; i < nodes.length; i++) {
                if (codama.isNode(nodes[i], "structFieldTypeNode") && nodes[i].type.name === "podU16") {
                    return true;
                }
            }

            return false;
        },
        transform: (node) => {
            codama.assertIsNode(node, ["structFieldTypeNode", "definedTypeLinkNode"]);
            return {
                ...node,
                type: codama.numberTypeNode("u16"),
            };
        },
    },
    {
        // PodBool -> bool
        select: (nodes) => {
            for (let i = 0; i < nodes.length; i++) {
                if (codama.isNode(nodes[i], "structFieldTypeNode") && nodes[i].type.name === "podBool") {
                    return true;
                }
            }

            return false;
        },
        transform: (node) => {
            codama.assertIsNode(node, ["structFieldTypeNode", "definedTypeLinkNode"]);
            return {
                ...node,
                type: codama.numberTypeNode("bool"),
            };
        },
    },
    // add 8 byte discriminator to accountNode
    {
        select: '[accountNode]',
        transform: (node) => {
            codama.assertIsNode(node, "accountNode");

            return {
                ...node,
                data: {
                    ...node.data,
                    fields: [
                        codama.structFieldTypeNode({ name: 'discriminator', type: codama.numberTypeNode('u64') }),
                        ...node.data.fields
                    ]
                }
            };
        },
    },
]));

const traitOptions = {
    baseDefaults: [
        'borsh::BorshSerialize',
        'borsh::BorshDeserialize',
        'serde::Serialize',
        'serde::Deserialize',
        'Clone',
        'Debug',
        'Eq',
        'PartialEq',
    ],
    dataEnumDefaults: [],
    scalarEnumDefaults: ['Copy', 'Hash', 'num_derive::FromPrimitive', 'clap::ValueEnum'],
    structDefaults: [],
};

weightTableKinobi.accept(renderers.renderRustVisitor(path.join(rustWeightTableClientDir, "src", "generated"), {
    formatCode: true,
    crateFolder: rustWeightTableClientDir,
    deleteFolderBeforeRendering: true,
    toolchain: "+nightly-2024-07-25",
    traitOptions,
}));
weightTableKinobi.accept(renderers.renderJavaScriptVisitor(path.join(jsWeightTableClientDir), {}));
