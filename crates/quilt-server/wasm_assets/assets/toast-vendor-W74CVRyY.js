import{r as o}from"./react-vendor-3e09n0Lt.js";import{h as b,u as L,a as v,m as R}from"./vendor-misc-DFOIfwfl.js";var S=e=>typeof e=="function",E=(e,t)=>S(e)?e(t):e,F=(()=>{let e=0;return()=>(++e).toString()})(),P=(()=>{let e;return()=>{if(e===void 0&&typeof window<"u"){let t=matchMedia("(prefers-reduced-motion: reduce)");e=!t||t.matches}return e}})(),U=20,C="default",N=(e,t)=>{let{toastLimit:a}=e.settings;switch(t.type){case 0:return{...e,toasts:[t.toast,...e.toasts].slice(0,a)};case 1:return{...e,toasts:e.toasts.map(r=>r.id===t.toast.id?{...r,...t.toast}:r)};case 2:let{toast:i}=t;return N(e,{type:e.toasts.find(r=>r.id===i.id)?1:0,toast:i});case 3:let{toastId:s}=t;return{...e,toasts:e.toasts.map(r=>r.id===s||s===void 0?{...r,dismissed:!0,visible:!1}:r)};case 4:return t.toastId===void 0?{...e,toasts:[]}:{...e,toasts:e.toasts.filter(r=>r.id!==t.toastId)};case 5:return{...e,pausedAt:t.time};case 6:let n=t.time-(e.pausedAt||0);return{...e,pausedAt:void 0,toasts:e.toasts.map(r=>({...r,pauseDuration:r.pauseDuration+n}))}}},k=[],j={toasts:[],pausedAt:void 0,settings:{toastLimit:U}},y={},M=(e,t=C)=>{y[t]=N(y[t]||j,e),k.forEach(([a,i])=>{a===t&&i(y[t])})},T=e=>Object.keys(y).forEach(t=>M(e,t)),B=e=>Object.keys(y).find(t=>y[t].toasts.some(a=>a.id===e)),$=(e=C)=>t=>{M(t,e)},Y={blank:4e3,error:4e3,success:2e3,loading:1/0,custom:4e3},q=(e={},t=C)=>{let[a,i]=o.useState(y[t]||j),s=o.useRef(y[t]);o.useEffect(()=>(s.current!==y[t]&&i(y[t]),k.push([t,i]),()=>{let r=k.findIndex(([u])=>u===t);r>-1&&k.splice(r,1)}),[t]);let n=a.toasts.map(r=>{var u,g,h;return{...e,...e[r.type],...r,removeDelay:r.removeDelay||((u=e[r.type])==null?void 0:u.removeDelay)||e?.removeDelay,duration:r.duration||((g=e[r.type])==null?void 0:g.duration)||e?.duration||Y[r.type],style:{...e.style,...(h=e[r.type])==null?void 0:h.style,...r.style}}});return{...a,toasts:n}},G=(e,t="blank",a)=>({createdAt:Date.now(),visible:!0,dismissed:!1,type:t,ariaProps:{role:"status","aria-live":"polite"},message:e,pauseDuration:0,...a,id:a?.id||F()}),x=e=>(t,a)=>{let i=G(t,e,a);return $(i.toasterId||B(i.id))({type:2,toast:i}),i.id},d=(e,t)=>x("blank")(e,t);d.error=x("error"),d.success=x("success"),d.loading=x("loading"),d.custom=x("custom"),d.dismiss=(e,t)=>{let a={type:3,toastId:e};t?$(t)(a):T(a)},d.dismissAll=e=>d.dismiss(void 0,e),d.remove=(e,t)=>{let a={type:4,toastId:e};t?$(t)(a):T(a)},d.removeAll=e=>d.remove(void 0,e),d.promise=(e,t,a)=>{let i=d.loading(t.loading,{...a,...a?.loading});return typeof e=="function"&&(e=e()),e.then(s=>{let n=t.success?E(t.success,s):void 0;return n?d.success(n,{id:i,...a,...a?.success}):d.dismiss(i),s}).catch(s=>{let n=t.error?E(t.error,s):void 0;n?d.error(n,{id:i,...a,...a?.error}):d.dismiss(i)}),e};var J=1e3,K=(e,t="default")=>{let{toasts:a,pausedAt:i}=q(e,t),s=o.useRef(new Map).current,n=o.useCallback((l,m=J)=>{if(s.has(l))return;let c=setTimeout(()=>{s.delete(l),r({type:4,toastId:l})},m);s.set(l,c)},[]);o.useEffect(()=>{if(i)return;let l=Date.now(),m=a.map(c=>{if(c.duration===1/0)return;let w=(c.duration||0)+c.pauseDuration-(l-c.createdAt);if(w<0){c.visible&&d.dismiss(c.id);return}return setTimeout(()=>d.dismiss(c.id,t),w)});return()=>{m.forEach(c=>c&&clearTimeout(c))}},[a,i,t]);let r=o.useCallback($(t),[t]),u=o.useCallback(()=>{r({type:5,time:Date.now()})},[r]),g=o.useCallback((l,m)=>{r({type:1,toast:{id:l,height:m}})},[r]),h=o.useCallback(()=>{i&&r({type:6,time:Date.now()})},[i,r]),p=o.useCallback((l,m)=>{let{reverseOrder:c=!1,gutter:w=8,defaultPosition:A}=m||{},z=a.filter(f=>(f.position||A)===(l.position||A)&&f.height),H=z.findIndex(f=>f.id===l.id),O=z.filter((f,I)=>I<H&&f.visible).length;return z.filter(f=>f.visible).slice(...c?[O+1]:[0,O]).reduce((f,I)=>f+(I.height||0)+w,0)},[a]);return o.useEffect(()=>{a.forEach(l=>{if(l.dismissed)n(l.id,l.removeDelay);else{let m=s.get(l.id);m&&(clearTimeout(m),s.delete(l.id))}})},[a,n]),{toasts:a,handlers:{updateHeight:g,startPause:u,endPause:h,calculateOffset:p}}},Q=b`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
 transform: scale(1) rotate(45deg);
  opacity: 1;
}`,V=b`
from {
  transform: scale(0);
  opacity: 0;
}
to {
  transform: scale(1);
  opacity: 1;
}`,W=b`
from {
  transform: scale(0) rotate(90deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(90deg);
	opacity: 1;
}`,X=v("div")`
  width: 20px;
  opacity: 0;
  height: 20px;
  border-radius: 10px;
  background: ${e=>e.primary||"#ff4b4b"};
  position: relative;
  transform: rotate(45deg);

  animation: ${Q} 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
  animation-delay: 100ms;

  &:after,
  &:before {
    content: '';
    animation: ${V} 0.15s ease-out forwards;
    animation-delay: 150ms;
    position: absolute;
    border-radius: 3px;
    opacity: 0;
    background: ${e=>e.secondary||"#fff"};
    bottom: 9px;
    left: 4px;
    height: 2px;
    width: 12px;
  }

  &:before {
    animation: ${W} 0.15s ease-out forwards;
    animation-delay: 180ms;
    transform: rotate(90deg);
  }
`,Z=b`
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
`,_=v("div")`
  width: 12px;
  height: 12px;
  box-sizing: border-box;
  border: 2px solid;
  border-radius: 100%;
  border-color: ${e=>e.secondary||"#e0e0e0"};
  border-right-color: ${e=>e.primary||"#616161"};
  animation: ${Z} 1s linear infinite;
`,ee=b`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(45deg);
	opacity: 1;
}`,te=b`
0% {
	height: 0;
	width: 0;
	opacity: 0;
}
40% {
  height: 0;
	width: 6px;
	opacity: 1;
}
100% {
  opacity: 1;
  height: 10px;
}`,ae=v("div")`
  width: 20px;
  opacity: 0;
  height: 20px;
  border-radius: 10px;
  background: ${e=>e.primary||"#61d345"};
  position: relative;
  transform: rotate(45deg);

  animation: ${ee} 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
  animation-delay: 100ms;
  &:after {
    content: '';
    box-sizing: border-box;
    animation: ${te} 0.2s ease-out forwards;
    opacity: 0;
    animation-delay: 200ms;
    position: absolute;
    border-right: 2px solid;
    border-bottom: 2px solid;
    border-color: ${e=>e.secondary||"#fff"};
    bottom: 6px;
    left: 6px;
    height: 10px;
    width: 6px;
  }
`,re=v("div")`
  position: absolute;
`,ie=v("div")`
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  min-width: 20px;
  min-height: 20px;
`,oe=b`
from {
  transform: scale(0.6);
  opacity: 0.4;
}
to {
  transform: scale(1);
  opacity: 1;
}`,se=v("div")`
  position: relative;
  transform: scale(0.6);
  opacity: 0.4;
  min-width: 20px;
  animation: ${oe} 0.3s 0.12s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
`,ne=({toast:e})=>{let{icon:t,type:a,iconTheme:i}=e;return t!==void 0?typeof t=="string"?o.createElement(se,null,t):t:a==="blank"?null:o.createElement(ie,null,o.createElement(_,{...i}),a!=="loading"&&o.createElement(re,null,a==="error"?o.createElement(X,{...i}):o.createElement(ae,{...i})))},le=e=>`
0% {transform: translate3d(0,${e*-200}%,0) scale(.6); opacity:.5;}
100% {transform: translate3d(0,0,0) scale(1); opacity:1;}
`,de=e=>`
0% {transform: translate3d(0,0,-1px) scale(1); opacity:1;}
100% {transform: translate3d(0,${e*-150}%,-1px) scale(.6); opacity:0;}
`,ce="0%{opacity:0;} 100%{opacity:1;}",pe="0%{opacity:1;} 100%{opacity:0;}",me=v("div")`
  display: flex;
  align-items: center;
  background: #fff;
  color: #363636;
  line-height: 1.3;
  will-change: transform;
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.1), 0 3px 3px rgba(0, 0, 0, 0.05);
  max-width: 350px;
  pointer-events: auto;
  padding: 8px 10px;
  border-radius: 8px;
`,ue=v("div")`
  display: flex;
  justify-content: center;
  margin: 4px 10px;
  color: inherit;
  flex: 1 1 auto;
  white-space: pre-line;
`,fe=(e,t)=>{let a=e.includes("top")?1:-1,[i,s]=P()?[ce,pe]:[le(a),de(a)];return{animation:t?`${b(i)} 0.35s cubic-bezier(.21,1.02,.73,1) forwards`:`${b(s)} 0.4s forwards cubic-bezier(.06,.71,.55,1)`}},ye=o.memo(({toast:e,position:t,style:a,children:i})=>{let s=e.height?fe(e.position||t||"top-center",e.visible):{opacity:0},n=o.createElement(ne,{toast:e}),r=o.createElement(ue,{...e.ariaProps},E(e.message,e));return o.createElement(me,{className:e.className,style:{...s,...a,...e.style}},typeof i=="function"?i({icon:n,message:r}):o.createElement(o.Fragment,null,n,r))});R(o.createElement);var he=({id:e,className:t,style:a,onHeightUpdate:i,children:s})=>{let n=o.useCallback(r=>{if(r){let u=()=>{let g=r.getBoundingClientRect().height;i(e,g)};u(),new MutationObserver(u).observe(r,{subtree:!0,childList:!0,characterData:!0})}},[e,i]);return o.createElement("div",{ref:n,className:t,style:a},s)},be=(e,t)=>{let a=e.includes("top"),i=a?{top:0}:{bottom:0},s=e.includes("center")?{justifyContent:"center"}:e.includes("right")?{justifyContent:"flex-end"}:{};return{left:0,right:0,display:"flex",position:"absolute",transition:P()?void 0:"all 230ms cubic-bezier(.21,1.02,.73,1)",transform:`translateY(${t*(a?1:-1)}px)`,...i,...s}},ge=L`
  z-index: 9999;
  > * {
    pointer-events: auto;
  }
`,D=16,ve=({reverseOrder:e,position:t="top-center",toastOptions:a,gutter:i,children:s,toasterId:n,containerStyle:r,containerClassName:u})=>{let{toasts:g,handlers:h}=K(a,n);return o.createElement("div",{"data-rht-toaster":n||"",style:{position:"fixed",zIndex:9999,top:D,left:D,right:D,bottom:D,pointerEvents:"none",...r},className:u,onMouseEnter:h.startPause,onMouseLeave:h.endPause},g.map(p=>{let l=p.position||t,m=h.calculateOffset(p,{reverseOrder:e,gutter:i,defaultPosition:t}),c=be(l,m);return o.createElement(he,{id:p.id,key:p.id,onHeightUpdate:h.updateHeight,className:p.visible?ge:"",style:c},p.type==="custom"?E(p.message,p):s?s(p):o.createElement(ye,{toast:p,position:l}))}))},xe=d;export{ve as F,d as n,xe as z};
