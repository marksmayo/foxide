local utils = require "mp.utils"
local msg = require "mp.msg"
local result = {completed=0, errors={}}
local finished = false
local function finish()
    if finished then return end
    finished = true
    local file = assert(io.open(os.getenv("FOXIDE_CONTROL_RESULTS"), "wb"))
    file:write(utils.format_json(result)); file:close()
    mp.commandv("quit", #result.errors == 0 and "0" or "1")
end
mp.add_timeout(12, function()
    table.insert(result.errors, "deadline before control completion")
    finish()
end)
mp.register_event("file-loaded", function()
    mp.add_timeout(0.2, function()
        local file = assert(io.open(os.getenv("FOXIDE_CONTROL_REQUESTS"), "rb"))
        local requests = assert(utils.parse_json(file:read("*a"))); file:close()
        local function step(index)
            if index > #requests then finish(); return end
            local request = requests[index]
            msg.info("control-probe dispatch " .. index .. " " .. request.name)
            local ok, err = mp.commandv(unpack(request.command))
            msg.info("control-probe returned " .. index .. " " .. tostring(ok) .. " " .. tostring(err))
            if not ok then table.insert(result.errors, request.name .. ": " .. tostring(err)) end
            local deadline = mp.get_time() + 2
            local function observe()
                local actual = request.property ~= "" and mp.get_property_native(request.property)
                if request.property ~= "" and actual ~= request.expected then
                    if mp.get_time() < deadline then mp.add_timeout(0.01, observe); return end
                    table.insert(result.errors, request.name .. " property mismatch: " .. tostring(actual))
                end
                if request.property == "secondary-sid" and request.expected == 2 and mp.get_property_native("sid") ~= 1 then
                    table.insert(result.errors, "Secondary selection replaced the primary subtitle")
                end
                result.completed = result.completed + 1
                mp.add_timeout(0.01, function() step(index + 1) end)
            end
            observe()
        end
        step(1)
    end)
end)
