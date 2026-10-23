-- Controlled additions to each fresh, isolated parity.db copy, never to a shared DB.
-- The live parity capture has no labels/members and an empty second column.
CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT OR IGNORE INTO kv(key,value) VALUES('members:7283','[{"id":7381,"displayName":"Behavior fixture member","email":"behavior@example.invalid"}]');
INSERT OR IGNORE INTO Label(id,projectId,value,raw_json) VALUES('behavior-7381',7283,'Behavior fixture tag','{"id":"behavior-7381","projectId":7283,"name":"Behavior fixture tag","value":"Behavior fixture tag"}');
INSERT OR IGNORE INTO Task(id,title,projectId,status,sectionId,section,updatedAt,raw_json)
SELECT 7381001,'Behavior fixture in Doing',7283,'Normal',20111,'Doing','2026-10-23T00:00:00Z',
'{"id":7381001,"title":"Behavior fixture in Doing","projectId":7283,"status":"Normal","sectionId":20111,"section":"Doing","description":"","updatedAt":"2026-10-23T00:00:00Z","uniqueIndex":7381001,"ticketNumber":"BEHAV-1","assignees":[],"labels":[]}'
WHERE NOT EXISTS(SELECT 1 FROM Task WHERE projectId=7283 AND sectionId=20111 AND status='Normal');
-- Complete the other inbox task from its captured public notification snapshot.
INSERT OR IGNORE INTO Task(id,title,projectId,status,sectionId,section,updatedAt,raw_json)
SELECT json_extract(raw_json,'$.task.id'),json_extract(raw_json,'$.task.title'),json_extract(raw_json,'$.task.projectId'),
  COALESCE(json_extract(raw_json,'$.task.status'),'Normal'),json_extract(raw_json,'$.task.sectionId'),json_extract(raw_json,'$.task.section'),json_extract(raw_json,'$.task.updatedAt'),
  json_set(json_extract(raw_json,'$.task'),'$.status',COALESCE(json_extract(raw_json,'$.task.status'),'Normal'),'$.description',COALESCE(json_extract(raw_json,'$.task.description'),''))
FROM Notification WHERE json_type(raw_json,'$.task')='object' AND json_extract(raw_json,'$.task.id')>0;
-- Placement must be exercised with a real rendered existing comment, not an empty list.
INSERT OR IGNORE INTO Comment(id,taskId,kind,text,commentText,createdAt,raw_json) VALUES
(7381002,60253,'qa','<p>Existing behavior fixture comment</p>','Existing behavior fixture comment','2026-10-22T12:00:00Z','{"id":7381002,"taskId":60253,"kind":"qa","text":"<p>Existing behavior fixture comment</p>","commentText":"Existing behavior fixture comment","createdAt":"2026-10-22T12:00:00Z","creator":{"displayName":"Behavior fixture author"}}');
UPDATE Task SET totalComments=1,raw_json=json_set(raw_json,'$.commentCount',1,'$.totalComments',1) WHERE id=60253;
