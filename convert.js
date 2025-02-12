import fs from "fs";

// TODO: handle remap!
for (const jsFilename of process.argv.slice(2)) {
    const jsonFilename = jsFilename.replace(/\.js$/i, '') + '.json';
    console.log(jsFilename, '->', jsonFilename);
    const js = fs.readFileSync(jsFilename, { encoding: 'UTF-8' });
    new Function('CanvasCycle', js)({
        processImage(data) {
            fs.writeFileSync(jsonFilename, JSON.stringify(data), {
                encoding: 'utf-8'
            });
        },
        initScene(data) {
            fs.writeFileSync(jsonFilename, JSON.stringify(data), {
                encoding: 'utf-8'
            });
        }
    });
}
