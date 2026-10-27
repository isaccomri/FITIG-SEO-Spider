"""Interact only with a Windows dialog belonging to the native test's owned app PID."""
import ctypes as C,json,sys,time
from ctypes import wintypes as W
pid=int(sys.argv[1]); action=sys.argv[2]
assert pid>0 and action in {'inspect','cancel','select','overwrite-no','overwrite-yes'}
u=C.WinDLL('user32',use_last_error=True)
callback=C.WINFUNCTYPE(W.BOOL,W.HWND,W.LPARAM)
u.EnumWindows.argtypes=[callback,W.LPARAM];u.EnumChildWindows.argtypes=[W.HWND,callback,W.LPARAM]
u.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
u.GetClassNameW.argtypes=[W.HWND,W.LPWSTR,C.c_int];u.GetWindowTextW.argtypes=[W.HWND,W.LPWSTR,C.c_int]
u.GetDlgItem.argtypes=[W.HWND,C.c_int];u.GetDlgItem.restype=W.HWND
u.GetDlgCtrlID.argtypes=[W.HWND];u.GetDlgCtrlID.restype=C.c_int
u.IsWindowVisible.argtypes=[W.HWND];u.IsWindowVisible.restype=W.BOOL
u.PostMessageW.argtypes=[W.HWND,W.UINT,W.WPARAM,W.LPARAM]
u.SendMessageTimeoutW.argtypes=[W.HWND,W.UINT,W.WPARAM,W.LPARAM,W.UINT,W.UINT,C.POINTER(C.c_size_t)]
def info(h):
 cls=C.create_unicode_buffer(256);title=C.create_unicode_buffer(2048)
 u.GetClassNameW(h,cls,256);u.GetWindowTextW(h,title,2048)
 return {'handle':int(h),'class':cls.value,'title':title.value,'id':u.GetDlgCtrlID(h),'visible':bool(u.IsWindowVisible(h))}
def windows():
 result=[]
 @callback
 def collect(h,_):
  owner=W.DWORD();u.GetWindowThreadProcessId(h,C.byref(owner))
  if owner.value==pid and u.IsWindowVisible(h):result.append(info(h))
  return True
 u.EnumWindows(collect,0);return result
deadline=time.monotonic()+15;dialog=None
while time.monotonic()<deadline:
 candidates=[x for x in windows() if x['class']=='#32770']
 if action.startswith('overwrite'):
  candidates=[x for x in candidates if not x['title'].startswith('FITIG -')]
 else:candidates=[x for x in candidates if x['title'].startswith('FITIG -')]
 if candidates:dialog=candidates[-1];break
 time.sleep(.05)
if dialog is None:raise RuntimeError('Owned file dialog not found: '+json.dumps(windows(),ensure_ascii=False))
h=dialog['handle'];children=[]
@callback
def child(c,_):children.append(info(c));return True
u.EnumChildWindows(h,child,0)
if action=='inspect':print(json.dumps({'dialog':dialog,'children':children},ensure_ascii=False));sys.exit(0)
if action=='select':
 text=C.create_unicode_buffer(sys.argv[3])
 edit=None
 while time.monotonic()<deadline and not edit:
  owner=W.DWORD();u.GetWindowThreadProcessId(h,C.byref(owner))
  if owner.value!=pid or not u.IsWindowVisible(h):raise RuntimeError('Owned file dialog disappeared before filename readiness')
  children.clear();u.EnumChildWindows(h,child,0)
  combo=u.GetDlgItem(h,1148);candidate=u.GetDlgItem(h,1152)
  if candidate and u.IsWindowVisible(candidate):edit=candidate
  if combo:
   edits=[]
   @callback
   def find(c,_):
    data=info(c)
    if data['class']=='Edit' and data['visible']:edits.append(c)
    return True
   u.EnumChildWindows(combo,find,0)
   if len(edits)==1:edit=edits[0]
  if not edit:
   edits=[x for x in children if x['class']=='Edit' and x['id']==1001 and x['visible']]
   if len(edits)==1:edit=edits[0]['handle']
  if not edit:time.sleep(.05)
 if not edit:raise RuntimeError('Owned filename edit missing: '+json.dumps(children,ensure_ascii=False))
 result=C.c_size_t()
 if not u.SendMessageTimeoutW(edit,0x000c,0,C.cast(text,C.c_void_p).value,2,3000,C.byref(result)):raise RuntimeError('Filename control timed out')
 u.PostMessageW(h,0x111,1,0)
elif action=='cancel':u.PostMessageW(h,0x111,2,0)
else:
 wanted=7 if action=='overwrite-no' else 6
 buttons=[x for x in children if x['class']=='Button' and x['id']==wanted and x['visible']]
 if not buttons:
  title='&No' if action=='overwrite-no' else '&Yes'
  buttons=[x for x in children if x['class']=='Button' and x['title']==title and x['visible']]
 if len(buttons)!=1:raise RuntimeError('Owned overwrite button missing: '+json.dumps({'dialog':dialog,'children':children},ensure_ascii=False))
 u.PostMessageW(buttons[0]['handle'],0xf5,0,0)
 deadline=time.monotonic()+3
 while time.monotonic()<deadline and any(x['handle']==h for x in windows()):time.sleep(.02)
 if any(x['handle']==h for x in windows()):raise RuntimeError('Owned overwrite prompt did not close')
print(json.dumps({'action':action,'dialog':dialog['title']},ensure_ascii=False))
