import { default as tippyJS, followCursor } from 'tippy.js';
import 'tippy.js/dist/tippy.css';

export function tippy(node, content) {
	if (!content) return;
	tippyJS(node, { content });
};

export function tippyFollow(node, content) {
	if (!content) return;
	tippyJS(node, {
		content,
		followCursor: true,
		plugins: [followCursor]
	});
};