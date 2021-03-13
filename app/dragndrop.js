const DROP_TYPE_GMA = 1;
const DROP_TYPE_ZIP = 2;
const DROP_TYPE_FOLDER = 3;
const DROP_TYPE_IMAGE = 4;

const RE_FILE_EXTENSION = /^.*?(?:\.(.*?))?$/;
function getFileExtension(name) {
	return name.match(RE_FILE_EXTENSION)?.[1];
}

function checkDrop(e) {
	if (e.dataTransfer?.files?.length > 0) {
		const currentDrag = [];
		let dropType;
		for (let i = 0; i < e.dataTransfer.files.length; i++) {
			const file = e.dataTransfer.files[i];
			const extension = getFileExtension(file.name.toLowerCase());
			let thisDropType;
			switch(extension) {
				case 'gma':
					thisDropType = DROP_TYPE_GMA;
					break;

				case null:
					if (!file.type || file.type.trim().length === 0) {
						thisDropType = DROP_TYPE_FOLDER;
						break;
					} else return [false];

				default:
					switch(file.type) {
						case 'application/zip':
						case 'application/x-zip-compressed':
						case 'multipart/x-zip':
							thisDropType = DROP_TYPE_ZIP;
							break;

						case 'application/octet-stream':
							if (extension === "zip") {
								thisDropType = DROP_TYPE_ZIP;
								break;
							}

						default:
							return [false];
					}
			}

			if (dropType) {
				if (dropType !== thisDropType) return [false];
			} else
				dropType = thisDropType;

			currentDrag.push(file);
		}
		return [true, currentDrag, dropType];
	}
	return [null];
}

let dragClock = 0;

window.addEventListener('dragenter', e => {
	dragClock++;
	e.stopPropagation();
	e.preventDefault();
	e.dataTransfer.dropEffect = 'copy';
	if (dragClock === 1) {
		document.body.classList.add('upload');
	}
    return false;
});

window.addEventListener('dragover', e => {
    e.stopPropagation();
    e.preventDefault();
    return false;
});

window.addEventListener('dragleave', e => {
	dragClock--;
	if (dragClock === 0) {
		document.body.classList.remove('upload');
	}
	e.stopPropagation();
	e.preventDefault();
    return false;
});

window.addEventListener('drop', e => {
	dragClock--;
	e.stopPropagation();
	e.preventDefault();

	let dropResult = checkDrop(e);
	if (dropResult !== null) {
		console.log('drop', dropResult);
	}

	return false;
});

//window.addEventListener('dragleave', dragLeave);
//window.addEventListener('drop', drop);