# Spoken commands: comma-separated forms, matched whole, never with placeables.

voice-yes = yes, yeah, yep, ok, okay, sure, confirm, correct, affirmative
voice-no = no, nope, nah, cancel, refuse, negative
voice-negation = not, don't, do not, never, can't, cannot, won't

voice-stop = stop, stop listening, enough, be quiet, pause listening
voice-ordinal-1 = first, one
voice-ordinal-2 = second, two
voice-ordinal-3 = third, three
voice-ordinal-4 = fourth, four
voice-ordinal-5 = fifth, five
voice-ordinal-6 = sixth, six
voice-ordinal-7 = seventh, seven
voice-ordinal-8 = eighth, eight
voice-ordinal-9 = ninth, nine

voice-next = next, next slide, forward, go on
voice-previous = previous, previous slide, back, go back
voice-first = first slide, beginning, go to start
voice-last = last slide, end, go to end
voice-exit = exit, leave, quit, escape, exit presentation, quit presentation

voice-wake = oku, hey oku, okay oku, ok oku
voice-filler = please, um, uh, er, hmm, now, thanks, thank you, the, a, an
voice-option = option, number, item, choice

voice-verb-replace = replace, overwrite
voice-verb-merge = merge, combine
voice-verb-discard = discard, throw away, drop
voice-verb-delete = delete, remove, erase
voice-verb-use-api = use api, use the api, switch to api
voice-verb-install = install, update
voice-verb-quit = quit, close app, shut down
voice-verb-close = close
voice-verb-import = import

voice-say-question = { $question }
voice-say-option = Option { $n }: { $option }.
voice-say-confirm = To confirm, say “{ $verb }”, or say no.
voice-say-confirm-yes = To confirm, say yes, or say no.
voice-say-confirm-action = To { $action }, say yes, or say no.
voice-action-open = open the file
voice-action-save = save the presentation
voice-action-save-copy = save a copy
voice-action-move = move the presentation
voice-action-new-deck = start a new presentation
voice-action-copy-pack = copy the pack
voice-action-copy-theme = copy the theme
voice-action-edit-theme = change the theme
voice-action-change-setting = change the setting
voice-action-change-dictation = change dictation
voice-action-download = download the model
voice-action-select-agent = switch agents
voice-say-permission = The agent asks for a permission: say yes to allow once, or no.
voice-say-agent-refused = The agent refused the request.
voice-say-agent-stopped = The agent stopped.
voice-say-agent-no-start = The agent could not start.
voice-say-agent-no-tools = This tab's tools are unavailable.
