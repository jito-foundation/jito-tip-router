const fs = require('fs');
const path = require('path');

// Function to recursively find all .rs files
function findRustFiles(dir, fileList = []) {
    const files = fs.readdirSync(dir);

    files.forEach(file => {
        const filePath = path.join(dir, file);
        const stat = fs.statSync(filePath);

        if (stat.isDirectory()) {
            findRustFiles(filePath, fileList);
        } else if (path.extname(file) === '.rs') {
            fileList.push(filePath);
        }
    });

    return fileList;
}

// Function to replace text in a file
function replaceInFile(filePath, searchText, replaceText) {
    try {
        const content = fs.readFileSync(filePath, 'utf8');
        const updatedContent = content.replace(new RegExp(searchText, 'g'), replaceText);

        // Only write if content changed
        if (content !== updatedContent) {
            fs.writeFileSync(filePath, updatedContent, 'utf8');
            console.log(`Updated ${filePath}`);
        }
    } catch (err) {
        console.error(`Error processing ${filePath}:`, err);
    }
}

// Main execution
try {
    const rustDir = path.join(__dirname, '../clients/rust');
    const rustFiles = findRustFiles(rustDir);

    if (rustFiles.length === 0) {
        console.log('No .rs files found in', rustDir);
        process.exit(1);
    }

    // Replace text in each file
    const searchText = 'serde\\(with = "serde_with::As::<serde_with::Bytes>"\\)';
    const replaceText = 'serde(with = "serde_big_array::BigArray")';

    rustFiles.forEach(file => {
        replaceInFile(file, searchText, replaceText);
    });

    console.log('Finished processing', rustFiles.length, 'files');
} catch (err) {
    console.error('Script failed:', err);
    process.exit(1);
}
