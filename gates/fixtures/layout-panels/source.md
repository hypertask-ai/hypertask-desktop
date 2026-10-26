# Graphite panel source projection

Authority is the read-only web checkout `/home/valentin/projects/hypertask/src`, revision recorded below. This is browser measurement of a source projection, not an authenticated live-app screenshot. No desktop value supplies a CSS token. The native fixture uses 1440x900, with the 420px panel at the right edge. Coverage is welcome, delivered plain human/assistant messages, composer and the first nine shortcut rows. Unmeasured icon paths, menus, rich blocks and remaining shortcut groups are not claimed equivalent.

| Projection | Web authority (paths relative to web checkout) |
| --- | --- |
| Panel 420px default | `src/lib/configs/style.config.ts:2` |
| Panel height, flex layout, inner 1px #434343 border | `src/components/AI_CHAT/AI_Chat_Sidebar.tsx:237-273`, `tailwind.config.ts:87` |
| Graphite outer 1px #3a3f47 border | `src/styles/tailwindThemes/graphite.css:188-190` |
| Panel #1a1d21, input/human bubble #262a30, suggestion #363b44, shortcut sheet #141518 | `src/styles/tailwindThemes/graphite.css:25,31-32,43` |
| White #ffffff and muted #8e9093 | `src/styles/tailwindThemes/graphite.css:53-55` |
| IBM Plex Sans family | `src/styles/tailwindThemes/graphite.css:169-172`, `src/lib/fonts/ibmPlexSans.ts:3` |
| Font tokens 12/13/14/24px | `tailwind.config.ts:167-178` |
| Header 48px, 16px top/8px horizontal padding, 16px gap, 14px; title 500, 4px gap, 2px/4px padding, 18px chevron | `src/components/AI_CHAT/ChatHeader.tsx:383-423` |
| Header controls 8px gap and 16px icons | `src/components/AI_CHAT/ChatHeader.tsx:429-450,586-599` |
| Welcome fills transcript, 40px/16px padding, centered group, 8px gap, max-width 448px; chips 8px padding, 12px font, 4px radius | `src/components/AI_CHAT/WelcomeScreen.tsx:351-364` |
| Four fixed sample prompts from the live suggestion pool | `src/components/AI_CHAT/WelcomeScreen.tsx:68-80` |
| Message list 8px padding and 8px inter-message spacing | `src/components/AI_CHAT/MessageList.tsx:111-123` |
| Messages 12px, human intrinsic/right-aligned, assistant full width; body 4px/8px padding, 4px radius; delivered actions 2px top margin | `src/components/AI_CHAT/MessageItem.tsx:46-95` |
| Message content 14px, 1.5 line-height, paragraph 8px bottom margin | `src/styles/messageItem.scss:55-74` |
| Composer outer 8px padding; inner 8px padding/8px radius | `src/components/AI_CHAT/AI_Tiptap_Container.tsx:261,298-306` |
| Input 380px maximum; contenteditable 8px vertical padding, paragraph 14px/21px | `src/components/AI_CHAT/AI_Tiptap_Container.tsx:326-335`, `src/styles/tiptap.module.scss:45-53,200-204` |
| Action row 8px top padding, 8px gap; leading controls 12px; scope 13px | `src/components/AI_CHAT/AiChatComposerActionRow.tsx:74-90`, `src/components/AI_CHAT/ChatScopeChip.tsx:24-45`, `src/components/Global/ModelSelectorDropdown.tsx:488-518` |
| Send icon 16px | `src/components/AI_CHAT/SendMessageButton.tsx:62-65` |
| Shortcut sheet 26vw full viewport, 14px; search 24px/21.74px, 500 weight, top/horizontal 16px | `src/components/sidebars/keyboardShortcuts.tsx:21,128-140` |
| Shortcut content 16px horizontal; heading 40px top/12px bottom, 14px/21px, 500; row 8px bottom/gap; labels 60%, 14px/16.94px; keys 40%, 4px gap | `src/components/sidebars/keyboardShortcuts.tsx:22-30,155-169` |
| Keycaps monospace, 6px/4px padding, 1.5px horizontal margin, 2px radius, #4f5765 background, 25px minimum height, 12px/18px normal | `src/components/sidebars/keyboardShortcuts.tsx:177-179` (Tailwind preflight supplies kbd monospace) |
| First nine shortcut labels and keys | `src/lib/constants/shortcuts.ts:49-65` |

The composer controls projection measures their shared outer row and send icon, not feature-flag-specific control inventories. Message fixture text is neutral matched data (`Hello`, `Hello back`), not an invented product response. Native source tests do not send a request or imply authorized AI transport. Comparisons preserve the existing 1.5px box and 0.5 typography tolerances. A missing projected element is a difference. Existing frozen inbox/board/task fixtures, comparisons and budgets are untouched.

Web source revision: `003a872014c35eeba1ab21047cdfdf70160e654e`.

Native CI fixtures use the production Linux platform text system through `TestAppContext::build_with_text_system`, not GPUI's default no-op test font metrics. They paint real renderer elements in an isolated 1440x900 test window. This measures native layout and typography, not GPU screenshot equivalence. Windows/macOS retain their ordinary behavior-test contexts.
