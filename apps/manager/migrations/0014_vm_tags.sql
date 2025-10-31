-- Add tags to existing function VMs (idempotent - only updates if not already tagged)
UPDATE vm
SET tags = ARRAY['type:function']
WHERE name LIKE 'fn-%'
  AND NOT ('type:function' = ANY(tags));

-- Add tags to existing container VMs (idempotent - only updates if not already tagged)
UPDATE vm
SET tags = ARRAY['type:container']
WHERE name LIKE 'container-%'
  AND NOT ('type:container' = ANY(tags));
