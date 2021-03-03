import { default as tippyJS, followCursor } from 'tippy.js';
import 'tippy.js/dist/tippy.css';

export function tippy(node, content) {
	tippyJS(node, { content });
};

export function tippyFollow(node, content) {
	tippyJS(node, {
		content,
		followCursor: true,
		plugins: [followCursor]
	});
};