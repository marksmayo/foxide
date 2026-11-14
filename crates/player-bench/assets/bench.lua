-- Internal playback-restart proxies, not display timestamps. No screenshots/readback.
local mp = require 'mp'
local utils = require 'mp.utils'
local path = os.getenv('FOXIDE_BENCH_EVENTS')
local seeks = os.getenv('FOXIDE_BENCH_SEEKS') == '1'
local file = assert(io.open(path, 'w'))
local first = true
local pending = nil
local next_seek = 0
local targets = {0.10, 0.80, 0.25, 0.65, 0.35, 0.90, 0.15, 0.70, 0.45, 0.55}
local function emit(event)
    event.mpv_time_seconds = mp.get_time()
    file:write(utils.format_json(event) .. '\n')
    file:flush()
end
local function stats()
    emit({kind='stats', hwdec=mp.get_property('hwdec-current'),
        interop=mp.get_property('hwdec-interop'),
        decoder_dropped=mp.get_property_number('decoder-frame-drop-count'),
        output_dropped=mp.get_property_number('frame-drop-count'),
        delayed=mp.get_property_number('vo-delayed-frame-count'),
        position=mp.get_property_number('time-pos'), paused=mp.get_property_bool('pause'),
        speed=mp.get_property_number('speed'), vo=mp.get_property('current-vo')})
end
local function seek()
    local duration = mp.get_property_number('duration', 0)
    if duration <= 31 then
        emit({kind='seek-unavailable', reason='Seek fixture must exceed 31 seconds'})
        return
    end
    if next_seek > #targets then emit({kind='seeks-complete'}); return end
    local mode = next_seek == 0 and 'relative+keyframes' or 'absolute+exact'
    local target = next_seek == 0 and 30 or targets[next_seek] * duration
    pending = {kind='seek-restart-proxy', index=next_seek, mode=mode,
        target_seconds=target, requested_mp_time_seconds=mp.get_time()}
    emit({kind='seek-request', index=next_seek, mode=mode, target_seconds=target})
    mp.commandv('seek', target, mode)
    next_seek = next_seek + 1
end
mp.register_event('playback-restart', function()
    if first then
        first = false
        emit({kind='first-playback-restart-proxy', video=mp.get_property('vid')})
        if seeks then mp.add_timeout(2, seek) end
    elseif pending then
        pending.latency_ms = (mp.get_time() - pending.requested_mp_time_seconds) * 1000
        pending.landed_seconds = mp.get_property_number('time-pos')
        emit(pending)
        pending = nil
        mp.add_timeout(1, seek)
    end
end)
mp.register_event('end-file', function(e) emit({kind='end-file', reason=e.reason, error=e.error}) end)
mp.add_periodic_timer(1, stats)
mp.register_event('shutdown', function() stats(); file:close() end)
