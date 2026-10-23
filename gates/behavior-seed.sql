-- Controlled additions to each fresh, isolated parity.db copy, never to a shared DB.
-- The live parity capture has no labels/members and an empty second column.
CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT OR IGNORE INTO kv(key,value) VALUES('members:7283','[{"id":7381,"displayName":"Behavior fixture member","email":"behavior@example.invalid"}]');
INSERT OR IGNORE INTO Label(id,projectId,value,raw_json) VALUES('behavior-7381',7283,'Behavior fixture tag','{"id":"behavior-7381","projectId":7283,"name":"Behavior fixture tag","value":"Behavior fixture tag"}');
INSERT OR IGNORE INTO Task(id,title,projectId,status,sectionId,section,updatedAt,raw_json)
SELECT 7381001,'Behavior fixture in Doing',7283,'Normal',20111,'Doing','2026-10-23T00:00:00Z',
'{"id":7381001,"title":"Behavior fixture in Doing","projectId":7283,"status":"Normal","sectionId":20111,"section":"Doing","description":"","updatedAt":"2026-10-23T00:00:00Z","uniqueIndex":7381001,"ticketNumber":"BEHAV-1","assignees":[],"labels":[]}'
WHERE NOT EXISTS(SELECT 1 FROM Task WHERE projectId=7283 AND sectionId=20111 AND status='Normal');
