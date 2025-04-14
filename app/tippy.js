import { default as tippyJS, followCursor } from 'tippy.js';
import 'tippy.js/dist/tippy.css';

function updateTippy(node, content, follow) {
	if (node._tippy) node._tippy.destroy();
	if (content) {
		tippyJS(node,
			follow ? {
				content,
				followCursor: true,
				plugins: [followCursor],
				interactive: false,
			} : {
				content,
				interactive: false,
			}
		);
	}
}

export function tippy(node, content) {
	updateTippy(node, content, false);
	return {
		update(content) {
			updateTippy(node, content, false);
		}
	};
};

export function tippyFollow(node, content) {
	updateTippy(node, content, true);
	return {
		update(content) {
			updateTippy(node, content, true);
		}
	};
};
