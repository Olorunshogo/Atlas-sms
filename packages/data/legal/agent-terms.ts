import type { LegalDocument } from '../../types/legal-document';
import { SUPPORT, supportMailtoHref, supportTelHref } from '../support';

/*
 * The agent agreement, version 0.1, PLACEHOLDER.
 *
 * This text has not been through the review the customer terms had. It exists
 * so the agent signup can link to a real document and record a real consent
 * rather than collect a tick that refers to nothing, and it is deliberately
 * conservative: it states the relationship and the obligations that are already
 * true in the product, and promises nothing about money.
 *
 * Before agent recruitment opens, replace the sections below with the reviewed
 * agreement and raise `version` to "1.0". Raising the version is what makes
 * every consent recorded against 0.1 identifiable as consent to the draft.
 *
 * The commission basis is deliberately absent. The locked decision is a flat
 * naira amount per package banded by product, the code still runs a percentage,
 * and the two disagree. A rate written into an agent agreement cannot be
 * lowered afterwards, so it stays out until that is settled.
 *
 * No naira figure belongs in this file, for the same reason as the customer
 * terms: every rate lives in packages/pricing and is referenced, never copied.
 */
export const agentTerms: LegalDocument = {
  slug: 'agent-terms',
  title: 'Agent Agreement',
  subtitle:
    'Governing the relationship between the operator and its field agents. Draft, pending review.',
  version: '0.1',
  effectiveDate: 'Not yet in effect',
  lastRevised: '3 September 2026',

  revisions: [
    {
      version: '0.1',
      date: '3 September 2026',
      summary:
        'Placeholder draft. Not reviewed, not in effect, and superseded before agent recruitment opens.',
    },
  ],

  intro: [
    {
      type: 'note',
      content: [
        {
          type: 'strong',
          text: 'This is a draft.',
        },
        ' It has not been reviewed and is not in effect. It is published so that an agent application can record what was shown at the time it was made. The final agreement will replace it in full, and applicants will be asked to accept the reviewed version before any agent begins work.',
      ],
    },
    {
      type: 'paragraph',
      content: [
        'This Agreement governs the relationship between the operator ("we", "us") and a person approved to act as a field Agent ("Agent", "you"). It applies from the moment an application is approved and continues until either party ends it.',
      ],
    },
    {
      type: 'paragraph',
      content: [
        'An Agent is an independent contractor. Nothing in this Agreement creates employment, partnership, or the authority to bind the operator to any obligation.',
      ],
    },
  ],

  sections: [
    {
      id: 'application-and-approval',
      number: '1',
      heading: 'Application and Approval',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'An application creates a pending account, not an active one. The operator reviews each application and may approve or decline it at its discretion. No Agent may represent the operator, collect any payment, or hold themselves out as acting for the operator before approval.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The Agent shall provide accurate information at application, including a local government area, a home address, and a reachable telephone number, and shall keep those details current.',
          ],
        },
      ],
    },

    {
      id: 'the-agent-role',
      number: '2',
      heading: 'The Agent Role',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'An Agent introduces customers to the operator, takes orders in the field, and acts as the local point of contact for the customers they recruit. Orders are placed through the Platform and are fulfilled by the operator.',
          ],
        },
        {
          type: 'list',
          ordered: false,
          items: [
            [
              'Prices are set by the operator and published on the Platform. An Agent may not quote a different price, add a charge of their own, or agree a discount that the operator has not published.',
            ],
            [
              'An Agent may not collect cash outside the payment methods the operator supports.',
            ],
            [
              'An Agent may not make promises about delivery times, product availability, or quality beyond what the customer terms state.',
            ],
          ],
        },
      ],
    },

    {
      id: 'commission',
      number: '3',
      heading: 'Commission',
      blocks: [
        {
          type: 'note',
          content: [
            'To be completed before this Agreement takes effect. The commission basis, the point at which commission is earned, and the payout schedule will be stated here in full. Nothing in this draft entitles an Agent to any payment.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Commission is earned on completed and paid orders only. An order that is cancelled, refunded, or never paid for earns nothing, and any commission already credited in respect of it may be reversed.',
          ],
        },
      ],
    },

    {
      id: 'conduct',
      number: '4',
      heading: 'Conduct',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'An Agent shall act honestly and courteously toward customers, colleagues, and delivery personnel, shall carry identification when representing the operator, and shall present it on request.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The operator may suspend or terminate an Agent for dishonesty, misrepresentation of prices or terms, abusive conduct, collecting payment outside the supported methods, creating accounts or orders that are not genuine, or any conduct that puts customers or colleagues at risk.',
          ],
        },
      ],
    },

    {
      id: 'customer-data',
      number: '5',
      heading: 'Customer Data',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Customer details an Agent obtains through this role belong to the operator and are held in confidence. They may be used only to serve those customers through the Platform. They may not be sold, shared, retained after termination, or used for any other business.',
          ],
        },
      ],
    },

    {
      id: 'termination',
      number: '6',
      heading: 'Termination',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Either party may end this Agreement at any time on notice. Commission properly earned on completed and paid orders before termination remains payable. Obligations that by their nature survive termination, including those on customer data and confidentiality, continue.',
          ],
        },
      ],
    },

    {
      id: 'changes-to-this-agreement',
      number: '7',
      heading: 'Changes to this Agreement',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator may amend this Agreement. Material changes will be notified, and continued activity as an Agent after the effective date constitutes acceptance. An Agent who does not accept an amended Agreement may end this Agreement under the section above.',
          ],
        },
      ],
    },

    {
      id: 'governing-law',
      number: '8',
      heading: 'Governing Law',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'This Agreement is governed by the laws of the Federal Republic of Nigeria, and the courts of Kaduna State have exclusive jurisdiction over any dispute arising under it.',
          ],
        },
      ],
    },

    {
      id: 'contact',
      number: '9',
      heading: 'Contact',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Questions about this Agreement, or about an application, may be directed to:',
          ],
        },
        {
          type: 'contact',
          entries: [
            {
              label: 'Email',
              value: SUPPORT.supportEmail,
              href: supportMailtoHref,
            },
            {
              label: 'Phone',
              value: SUPPORT.phoneDisplay,
              href: supportTelHref,
            },
            { label: 'Business address', value: 'Kaduna, Nigeria' },
          ],
        },
      ],
    },
  ],

  closing:
    'Draft version 0.1. Not in effect. To be replaced by the reviewed Agent Agreement before agent recruitment opens.',
};
