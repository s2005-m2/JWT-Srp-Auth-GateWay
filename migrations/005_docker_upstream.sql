UPDATE proxy_routes 
SET upstream_address = 'arc_generater:7000'
WHERE path_prefix IN ('/api/', '/ws/');
