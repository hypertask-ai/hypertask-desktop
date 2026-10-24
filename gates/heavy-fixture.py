"""Deterministic, non-private heavy-account workload (no network or real account data)."""
import argparse
import copy
import hashlib
import json
from pathlib import Path


def write(path, value):
    path.write_text(json.dumps(value, separators=(',', ':')), encoding='utf-8')


def generate(out):
    out.mkdir(parents=True, exist_ok=True)
    demo = Path(__file__).resolve().parents[1] / 'fixtures/demo'
    load = lambda name: json.loads((demo / name).read_text())
    sections = load('sections-15.json')
    sections['sections'] = [s for s in sections['sections'] if not s.get('deleted') and s.get('visibility')]
    write(out / 'sections.json', sections)
    write(out / 'projects.json', load('projects.json'))
    template = load('task-58006.json')['tasks'][0]
    actors = [{'id': 700000 + i, 'displayName': f'Colleague {i:03}',
               'photoURL': f'https://memory.invalid/avatar/{i:03}-1024.png'} for i in range(160)]
    tasks = []
    for i in range(320):
        task = copy.deepcopy(template)
        section = sections['sections'][i % len(sections['sections'])]
        task.update(id=58006 if i == 0 else 800000+i, title=f'Heavy account delivery {i:03}: investigate rollout and customer feedback',
                    uniqueIndex=10000+i, ticketNumber=f'HEAVY-{10000+i}', sectionId=section['id'], section=section['section_title'],
                    assignees=[actors[i % 160], actors[(i+37) % 160]], sub_tasks=[],
                    description=''.join(f'<p>Task {i}, investigation {p}: ' +
                        'Review the customer report, reproduction steps, rollout metrics and follow-up actions. ' * 20 + '</p>'
                        for p in range(16)) + ''.join(
                        f'<p><img src="https://memory.invalid/task/{i}/capture-{n}-2560x1440.png" width="640" height="360"></p>'
                        for n in range(3)))
        tasks.append(task)
    write(out / 'tasks.json', {'success': True, 'tasks': tasks})
    notifications = []
    template_n = load('inbox.json')['user_notifications'][0]
    for i in range(640):
        n = copy.deepcopy(template_n)
        task = tasks[i % len(tasks)]
        n.update(id=900000+i, taskId=task['id'], projectId=15,
                 task={k: v for k, v in task.items() if k != 'description'},
                 fromUser=actors[i % 160], fromAgent=None, recentActors=[actors[i % 160]],
                 createdAt=f'2026-10-20T{(i//3600)%24:02}:{(i//60)%60:02}:{i%60:02}.000Z')
        notifications.append(n)
    write(out / 'inbox.json', {'success': True, 'user_notifications': notifications})
    comment = load('comments-58006.json')['comments'][0]
    comments = []
    for i in range(48):
        c = copy.deepcopy(comment)
        c.update(id=950000+i, taskId=58006, creator=actors[i],
                 text=f'<p>Review {i}: ' + 'Measured rollout details and customer follow-up. ' * 30 + '</p>',
                 createdAt=f'2026-10-20T10:{i:02}:00.000Z')
        comments.append(c)
    write(out / 'comments-58006.json', {'success': True, 'comments': comments})
    sessions = [{'id': f'heavy-session-{i}', 'title': f'Investigation {i}', 'messages': [
        {'role': 'human' if j % 2 == 0 else 'assistant',
         'content': f'Session {i} turn {j}: ' + 'Review the evidence and propose the next safe rollout step. ' * 24}
        for j in range(24)]} for i in range(40)]
    # Not an import response; consumed by the same AI history/transcript path as the server.
    write(out.parent / 'heavy-ai-history.json', sessions)
    manifest = {'Version': 1, 'InboxItems': len(notifications), 'BoardCards': len(tasks),
                'DescriptionBytes': sum(len(t['description'].encode()) for t in tasks),
                'DescriptionImages': 320*3, 'DistinctAvatars': len(actors), 'Comments': len(comments),
                'AiSessions': len(sessions), 'AiMessages': sum(len(s['messages']) for s in sessions),
                'AiHistoryBytes': (out.parent / 'heavy-ai-history.json').stat().st_size,
                'ImageSourceWidth': 2560, 'ImageSourceHeight': 1440,
                'ImageDisplayWidth': 640, 'ImageDisplayHeight': 360,
                'AvatarSourceWidth': 1024, 'AvatarSourceHeight': 1024}
    paths = sorted(out.glob('*.json')) + [out.parent / 'heavy-ai-history.json']
    manifest['FixtureSHA256'] = hashlib.sha256(b''.join(p.name.encode()+p.read_bytes() for p in paths)).hexdigest()
    write(out.parent / 'heavy-manifest.json', manifest)
    print('HEAVY_FIXTURE_GENERATED ' + json.dumps(manifest, sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    generate(parser.parse_args().out)
