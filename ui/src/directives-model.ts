import type {PageRecord} from './model';
export type DirectiveRule={source:string;agent:string;value:string;flags:string[]};
export function directiveRules(page:PageRecord):DirectiveRule[]|null{return Array.isArray(page.directive_rules)?page.directive_rules as DirectiveRule[]:null}
export function directiveRestriction(page:PageRecord,agent:string,flag:'noindex'|'nofollow'):boolean{const rules=directiveRules(page);if(rules)return rules.some(rule=>(rule.agent==='*'||rule.agent.toLowerCase()===agent.toLowerCase())&&rule.flags.some(value=>value===flag||value==='none'));return String(page.directives??'').split(/[,\s:]+/).some(value=>[flag,'none'].includes(value.toLowerCase()))}
export function directiveCell(page:PageRecord,agent:string,flag:'noindex'|'nofollow'){return directiveRules(page)===null?'':String(directiveRestriction(page,agent,flag))}
