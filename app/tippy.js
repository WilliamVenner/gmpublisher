import { default as tippyJS, followCursor } from 'tippy.js';
import 'tippy.js/dist/tippy.css';

export function tippy(node, content) {
	if (node._tippy) node._tippy.destroy();
	if (content) {
		tippyJS(node, { content });
	}
};

export function tippyFollow(node, content) {
	if (node._tippy) node._tippy.destroy();
	if (content) {
		tippyJS(node, {
			content,
			followCursor: true,
			plugins: [followCursor]
		});
	}
};
