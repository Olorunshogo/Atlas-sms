import type { LegalDocument } from '../../types/legal-document';
import { SUPPORT, supportMailtoHref, supportTelHref } from '../support';

/*
 * The customer Terms and Conditions, version 1.0.
 *
 * Data, not markup, so the same object serves the page, the consent record
 * written at signup, and anything later that needs the text without a browser.
 *
 * Two rules govern edits here.
 *
 * A revision that changes an obligation must raise `version`. Signup stores
 * this string against the account, so leaving it alone would make the record
 * claim the customer agreed to text they never saw.
 *
 * No naira figure appears anywhere in this document, and none may be added.
 * Clause 10.8 says the rescheduling fee equals the delivery fee in effect at
 * the time, which is a reference rather than a number, and every other money
 * rule is written the same way. A figure typed here is a second copy of a rate
 * that lives in packages/pricing, and it will drift from what is charged.
 */
export const buyerTerms: LegalDocument = {
  slug: 'buyer-terms',
  title: 'Terms and Conditions',
  subtitle:
    'Governing the use of the Platform, website, mobile application, and related services',
  version: '1.0',
  effectiveDate: '5 August 2026',
  lastRevised: '5 August 2026',

  revisions: [
    {
      version: '1.0',
      date: '5 August 2026',
      summary:
        'Initial publication of the official Terms and Conditions governing the Platform.',
    },
  ],

  intro: [
    {
      type: 'paragraph',
      content: [
        'These Terms and Conditions ("Terms") govern the access to and use of the website, mobile application, and related services (collectively, the "Platform") operated by the operator ("we", "us", or "our"). The operator is an agricultural commodity marketplace and delivery platform based in Kaduna, Nigeria, that enables customers to order food commodities and household essentials. The operator procures, fulfils, and delivers such orders within designated Delivery Zones.',
      ],
    },
    {
      type: 'paragraph',
      content: [
        'By accessing the Platform, creating an Account, submitting an Order, or otherwise using any Service, the Customer agrees to be bound by these Terms. If the Customer does not agree to these Terms, the Customer must not access or use the Platform or any Service.',
      ],
    },
    {
      type: 'paragraph',
      content: [
        'These Terms constitute a legally binding agreement between the Customer and the operator. They apply to all Customers, including individual consumers and Business Customers, and to all Orders placed through the Platform.',
      ],
    },
  ],

  sections: [
    {
      id: 'definitions',
      number: '2',
      heading: 'Definitions',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'In these Terms, unless the context otherwise requires, the following words and expressions shall have the meanings ascribed to them below. Words importing the singular include the plural and vice versa. References to "including" shall mean "including without limitation".',
          ],
        },
        {
          type: 'definition',
          term: 'Account',
          content: [
            'the unique customer account registered on the Platform by a Customer for the purpose of placing Orders and accessing the Services.',
          ],
        },
        {
          type: 'definition',
          term: 'Business Customer',
          content: [
            'a Customer that places Orders in a commercial or institutional capacity, including for recurring, standing, bulk, or corporate requirements.',
          ],
        },
        {
          type: 'definition',
          term: 'Business Day',
          content: [
            'any day other than a Saturday, Sunday, or public holiday in Kaduna State, Nigeria, on which commercial banks are open for ordinary business.',
          ],
        },
        {
          type: 'definition',
          term: 'Customer',
          content: [
            'any natural or legal person who accesses the Platform, creates an Account, submits an Order, or uses any Service.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery',
          content: [
            'the physical transportation and handover of Products ordered by a Customer to the delivery address specified in the Order Confirmation.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery Attempt',
          content: [
            'each occasion on which a Delivery Partner attends the designated delivery address for the purpose of completing a Delivery.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery Fee',
          content: [
            'the fee charged by the operator for the Delivery of Products under an Order, as displayed to the Customer prior to Order Confirmation.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery Partner',
          content: [
            'any individual engaged or authorised by the operator to carry out Delivery of Products, including employees, contractors, and third-party logistics personnel.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery Window',
          content: [
            'the period within which the operator targets or guarantees completion of Delivery, being a target of four (4) hours and a maximum of six (6) hours from Order Confirmation, subject to these Terms.',
          ],
        },
        {
          type: 'definition',
          term: 'Delivery Zone',
          content: [
            'a geographic area within which the operator currently provides Delivery services, as designated by the operator from time to time. Services currently apply only within Kaduna Delivery Zones.',
          ],
        },
        {
          type: 'definition',
          term: 'Force Majeure',
          content: [
            'any event or circumstance beyond the reasonable control of the operator, including natural disasters, flooding, heavy rain, war, riots, civil unrest, government action or restriction, road closures, fuel shortages, security incidents, public holidays, network or system outages, and any other cause that prevents or delays performance.',
          ],
        },
        {
          type: 'definition',
          term: 'Fulfillment',
          content: [
            'the process of procuring, verifying, packing, and preparing Products for Delivery following Order Confirmation.',
          ],
        },
        {
          type: 'definition',
          term: 'Order',
          content: [
            'a request submitted by a Customer through the Platform for the purchase and Delivery of specified Products.',
          ],
        },
        {
          type: 'definition',
          term: 'Order Confirmation',
          content: [
            'the formal acceptance by the operator of an Order Submission, following verification of payment, inventory, and price, which creates a binding contract between the Customer and the operator.',
          ],
        },
        {
          type: 'definition',
          term: 'Order Submission',
          content: [
            'the act of a Customer submitting a request for Products through the Platform. An Order Submission does not constitute Order Confirmation and does not create a binding contract.',
          ],
        },
        {
          type: 'definition',
          term: 'Platform',
          content: [
            'the website, mobile application, and any related digital interface through which Customers access the Services.',
          ],
        },
        {
          type: 'definition',
          term: 'Products',
          content: [
            'the food commodities, household essentials, and other goods made available for purchase through the Platform.',
          ],
        },
        {
          type: 'definition',
          term: 'Promotion',
          content: [
            'any promotional code, coupon, discount, referral reward, or other incentive offered by the operator subject to applicable terms.',
          ],
        },
        {
          type: 'definition',
          term: 'Proof of Delivery',
          content: [
            'the electronic signature, delivery photograph, or other evidence recorded by the Delivery Partner confirming completion of Delivery.',
          ],
        },
        {
          type: 'definition',
          term: 'Recurring Order',
          content: [
            'an Order scheduled to repeat at intervals specified by a Business Customer or other authorised Customer, each instance of which generates a new Order ID and Payment ID.',
          ],
        },
        {
          type: 'definition',
          term: 'Refund',
          content: [
            'the return of monies paid by a Customer in respect of an Order or part of an Order, processed in accordance with these Terms.',
          ],
        },
        {
          type: 'definition',
          term: 'Service or Services',
          content: [
            'the marketplace, procurement, Fulfillment, and Delivery services provided by the operator through the Platform.',
          ],
        },
        {
          type: 'definition',
          term: 'Service Level',
          content: [
            'the operational standards applicable to Delivery, including the target Delivery time of four (4) hours and the guaranteed Delivery Window of six (6) hours from Order Confirmation, subject to these Terms.',
          ],
        },
      ],
    },

    {
      id: 'acceptance-of-terms',
      number: '3',
      heading: 'Acceptance of Terms',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'By creating an Account, submitting an Order, or continuing to use the Platform, the Customer confirms that the Customer has read, understood, and agrees to be bound by these Terms and any policies referenced herein, including the Privacy Policy.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The operator may amend these Terms from time to time. Material changes shall be notified to Customers through the Platform, by email, or by other reasonable means. Continued use of the Platform after the effective date of any amendment constitutes acceptance of the amended Terms. If the Customer does not agree to the amended Terms, the Customer must cease using the Platform.',
          ],
        },
      ],
    },

    {
      id: 'eligibility',
      number: '4',
      heading: 'Eligibility',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The Platform and Services are available only to persons who are at least eighteen (18) years of age and who have legal capacity to enter into binding contracts under the laws of the Federal Republic of Nigeria. By using the Platform, the Customer represents and warrants that the Customer meets these eligibility requirements.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Business Customers represent and warrant that the individual placing Orders on their behalf has full authority to bind the Business Customer to these Terms.',
          ],
        },
      ],
    },

    {
      id: 'customer-accounts',
      number: '5',
      heading: 'Customer Accounts',
      children: [
        {
          id: 'registration',
          number: '5.1',
          heading: 'Registration',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'To place an Order, the Customer must register an Account by providing accurate, complete, and current information as requested during the registration process. The Customer shall promptly update any information that becomes inaccurate or incomplete.',
              ],
            },
          ],
        },
        {
          id: 'account-security',
          number: '5.2',
          heading: 'Account Security',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "The Customer is responsible for maintaining the confidentiality of Account credentials and for all activities that occur under the Account. The Customer shall notify the operator immediately of any unauthorised use of the Account or any other breach of security. The operator shall not be liable for any loss arising from the Customer's failure to maintain Account security.",
              ],
            },
          ],
        },
        {
          id: 'one-account-policy',
          number: '5.3',
          heading: 'One Account Policy',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Unless otherwise approved in writing by the operator, each Customer may maintain only one (1) active Account. The operator may, at its sole discretion, investigate and take action in respect of multiple Accounts associated with the same individual or entity, including suspension or Termination.',
              ],
            },
          ],
        },
        {
          id: 'suspension',
          number: '5.4',
          heading: 'Suspension',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator may suspend an Account, with or without notice, where the operator reasonably believes that the Account has been used in breach of these Terms, for fraudulent purposes, in connection with abusive conduct, or in a manner that poses a risk to the operator, its personnel, or other Customers.',
              ],
            },
          ],
        },
        {
          id: 'termination',
          number: '5.5',
          heading: 'Termination',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "The Customer may terminate the Account at any time by contacting the operator through the official support channels. The operator may terminate an Account where the Customer has committed a material breach of these Terms, where required by law, or where the operator ceases to provide the Services. Upon termination, the Customer's right to use the Platform ceases immediately. Provisions of these Terms that by their nature should survive termination shall continue in full force and effect.",
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'services',
      number: '6',
      heading: 'Services',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator operates an agricultural commodity marketplace and delivery platform. Through the Platform, Customers may browse and order food commodities and household essentials. The operator procures the Products, fulfils the Order, and arranges Delivery to the address specified by the Customer within a designated Delivery Zone.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Availability of the Services depends on the Delivery Zones designated by the operator from time to time. The operator currently operates only within Kaduna Delivery Zones. The operator reserves the right to expand, restrict, or modify Delivery Zones at its sole discretion and without prior notice, except that any Order already subject to Order Confirmation shall be fulfilled in accordance with the Delivery Zone applicable at the time of Order Confirmation.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            "The operator does not guarantee continuous, uninterrupted, or error-free access to the Platform. Temporary unavailability may occur due to maintenance, technical issues, or circumstances beyond the operator's reasonable control.",
          ],
        },
      ],
    },

    {
      id: 'orders',
      number: '7',
      heading: 'Orders',
      children: [
        {
          id: 'order-submission-and-confirmation',
          number: '7.1',
          heading: 'Order Submission and Order Confirmation',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "Order Submission is the Customer's request to purchase Products and does not constitute acceptance by the operator. Order Confirmation is issued only after successful payment verification, inventory validation, and any other internal verification required by the operator. A binding contract is formed only upon Order Confirmation. Where payment is successfully received and inventory is immediately confirmed, Order Confirmation may be issued automatically.",
              ],
            },
          ],
        },
        {
          id: 'contract-formation',
          number: '7.2',
          heading: 'Contract Formation',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The contract between the Customer and the operator is formed at the moment of Order Confirmation and comprises these Terms together with the particulars of the confirmed Order, including the Products, price, Delivery Fee, and delivery address.',
              ],
            },
          ],
        },
        {
          id: 'inventory-verification',
          number: '7.3',
          heading: 'Inventory Verification',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Prior to Order Confirmation, the operator shall verify the availability of the Products requested in the Order Submission. Where a Product is unavailable, the operator shall notify the Customer and process a Refund in respect of that Product before any discussion of substitution.',
              ],
            },
          ],
        },
        {
          id: 'payment-verification',
          number: '7.4',
          heading: 'Payment Verification',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Order Confirmation is conditional upon successful verification of payment. Where payment cannot be verified, the operator may decline the Order Submission without liability.',
              ],
            },
          ],
        },
        {
          id: 'price-verification',
          number: '7.5',
          heading: 'Price Verification',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Prices displayed on the Platform are subject to verification prior to Order Confirmation. Where the price of a Product has changed between Order Submission and Order Confirmation, the operator shall notify the Customer before Order Confirmation. The Customer may accept the revised price or cancel the affected portion of the Order without penalty, and the operator shall process a Refund in respect of that portion.',
              ],
            },
          ],
        },
        {
          id: 'cancellation-before-fulfillment',
          number: '7.6',
          heading: 'Cancellation before Fulfillment',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Customers may cancel an Order at any time before Fulfillment begins. Once Fulfillment has commenced, an Order cannot be cancelled except where required by applicable law or expressly approved by the operator. The operator shall confirm the commencement of Fulfillment through the Platform or other communication channel where practicable.',
              ],
            },
          ],
        },
        {
          id: 'order-modification',
          number: '7.7',
          heading: 'Order Modification',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Requests to modify an Order after Order Confirmation but before Fulfillment begins shall be considered by the operator on a case-by-case basis. The operator is under no obligation to accept a modification request. Any accepted modification shall be confirmed in writing or through the Platform.',
              ],
            },
          ],
        },
        {
          id: 'stock-availability',
          number: '7.8',
          heading: 'Stock Availability',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "Product availability is subject to inventory and supplier availability. Display of a Product on the Platform does not guarantee that the Product will be available at the time of Order Submission or Order Confirmation. Where a Product becomes unavailable before Fulfillment begins, the operator shall notify the Customer and issue a Refund for the unavailable Product. At the Customer's request, the operator may discuss suitable alternative Products. No substitute Product shall be supplied without the Customer's express approval.",
              ],
            },
          ],
        },
        {
          id: 'unavailable-products-and-partial-orders',
          number: '7.9',
          heading: 'Unavailable Products and Partial Orders',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator does not intentionally split Orders into multiple Deliveries due to Product unavailability. Products that are unavailable before Fulfillment shall be removed from the Order and refunded before Order Confirmation. Where only some Products in an Order are available, the operator may proceed with a partial Order in respect of the available Products, subject to notification to the Customer and Refund of the unavailable Products. The Customer may elect to cancel the entire Order before Fulfillment begins if a partial Order is not acceptable.',
              ],
            },
          ],
        },
        {
          id: 'substitute-products',
          number: '7.10',
          heading: 'Substitute Products',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "The operator shall not substitute any Product with another Product without the Customer's prior express approval. Where a Product is unavailable, a Refund shall be processed before any substitution is discussed. Any approved substitute shall be confirmed with the Customer prior to Fulfillment.",
              ],
            },
          ],
        },
        {
          id: 'recurring-orders',
          number: '7.11',
          heading: 'Recurring Orders',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Business Customers and other authorised Customers may set up Recurring Orders. Each instance of a Recurring Order generates a new Order ID and a new Payment ID. Each such instance is subject to these Terms as a separate Order, including verification of payment, inventory, and price.',
              ],
            },
          ],
        },
        {
          id: 'corporate-and-bulk-orders',
          number: '7.12',
          heading: 'Corporate and Bulk Orders',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Corporate and bulk Orders are subject to these Terms. The operator may require additional information, lead times, or commercial terms for large-volume Orders. Separate Order IDs and Payment IDs shall apply.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'payment',
      number: '8',
      heading: 'Payment',
      children: [
        {
          id: 'accepted-payment-methods',
          number: '8.1',
          heading: 'Accepted Payment Methods',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator accepts payment methods as displayed on the Platform from time to time, which may include bank transfer, card payment, and other electronic payment options. The Customer is responsible for ensuring that sufficient funds or credit are available.',
              ],
            },
          ],
        },
        {
          id: 'payment-timing',
          number: '8.2',
          heading: 'Payment Timing',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Payment is due at the time of Order Submission. Order Confirmation is conditional upon successful payment verification. The operator shall not commence Fulfillment until payment has been verified.',
              ],
            },
          ],
        },
        {
          id: 'failed-payments',
          number: '8.3',
          heading: 'Failed Payments',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Where payment fails or cannot be verified, the operator may decline the Order Submission. The Customer may resubmit the Order with a valid payment method.',
              ],
            },
          ],
        },
        {
          id: 'pricing-terms',
          number: '8.4',
          heading: 'Pricing',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Prices for Products and Delivery Fees are displayed on the Platform and are subject to verification prior to Order Confirmation. All prices are denominated in Nigerian Naira unless otherwise stated.',
              ],
            },
          ],
        },
        {
          id: 'taxes',
          number: '8.5',
          heading: 'Taxes',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Where applicable taxes, levies, or charges are imposed by law, such amounts shall be added to the Order total and disclosed to the Customer prior to Order Confirmation.',
              ],
            },
          ],
        },
        {
          id: 'refunds-payment',
          number: '8.6',
          heading: 'Refunds',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Refunds shall be processed in accordance with Section 14 of these Terms. Refunds shall ordinarily be made to the original payment method within a reasonable time after approval.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'fulfillment',
      number: '9',
      heading: 'Fulfillment',
      children: [
        {
          id: 'fulfillment-process',
          number: '9.1',
          heading: 'Fulfillment Process',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Upon Order Confirmation, the operator shall commence Fulfillment. Fulfillment includes procurement of the Products (where not already in stock), quality inspection, packing, and preparation for Delivery.',
              ],
            },
          ],
        },
        {
          id: 'commencement-of-fulfillment',
          number: '9.2',
          heading: 'Commencement of Fulfillment',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Fulfillment begins when the operator initiates procurement, packing, or preparation of the Products for the confirmed Order. From that point, the Customer may not cancel the Order except where required by applicable law or expressly approved by the operator.',
              ],
            },
          ],
        },
        {
          id: 'packing-and-quality-inspection',
          number: '9.3',
          heading: 'Packing and Quality Inspection',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator shall pack Products with reasonable care and shall conduct a quality inspection consistent with commercially reasonable standards for the type of Product. Perishable Products shall be handled in a manner appropriate to their nature.',
              ],
            },
          ],
        },
        {
          id: 'partial-fulfillment-and-inventory-shortage',
          number: '9.4',
          heading: 'Partial Fulfillment and Inventory Shortage',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "Where inventory shortage arises after Order Confirmation, the operator shall notify the Customer, process a Refund for the unavailable Products, and may proceed with partial Fulfillment of the available Products, subject to the Customer's rights under these Terms. The operator shall use reasonable efforts to maintain accurate inventory information. Occasional shortages may occur due to market conditions, supplier constraints, or other factors beyond the operator's reasonable control.",
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'delivery',
      number: '10',
      heading: 'Delivery',
      children: [
        {
          id: 'delivery-zones',
          number: '10.1',
          heading: 'Delivery Zones',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Delivery is available only within the Delivery Zones designated by the operator. The operator currently operates only within Kaduna Delivery Zones. The operator may expand, restrict, or reconfigure Delivery Zones at its sole discretion.',
              ],
            },
          ],
        },
        {
          id: 'delivery-schedule-and-window',
          number: '10.2',
          heading: 'Delivery Schedule and Window',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "The operator aims to complete Deliveries within four (4) hours of Order Confirmation. This is the target service level. Delivery may take up to six (6) hours depending on operational conditions. Delivery timelines may be extended where delays arise from severe weather, flooding, road closures, fuel shortages, civil disturbances, government restrictions, public holidays, security concerns, or other events beyond the operator's reasonable control. The operator shall use reasonable efforts to notify Customers of significant delays.",
              ],
            },
          ],
        },
        {
          id: 'address-verification',
          number: '10.3',
          heading: 'Address Verification',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Customers are responsible for providing accurate Delivery information, including addresses, landmarks, and contact details. Customers shall remain reachable throughout the Delivery process. Where Delivery cannot be completed because the Customer cannot be contacted or has provided inaccurate or incomplete information, the Delivery shall be treated as a failed Delivery and the Customer shall bear the cost of any subsequent Delivery Attempt. The operator may use mapping services, including Google Maps or equivalent, to assist with address verification.',
              ],
            },
          ],
        },
        {
          id: 'incorrect-address',
          number: '10.4',
          heading: 'Incorrect Address',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Where the delivery address provided by the Customer is incorrect, incomplete, or cannot reasonably be located, any resulting failed Delivery Attempt shall be treated as caused by the Customer. The Customer shall be liable for the applicable Delivery Fee for a second Delivery Attempt.',
              ],
            },
          ],
        },
        {
          id: 'customer-availability-and-communication',
          number: '10.5',
          heading: 'Customer Availability and Communication',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The Customer shall remain reachable during the Delivery Window by the contact telephone number associated with the Order. The Customer shall ensure that a person authorised to receive the Delivery is present at the delivery address. The operator and the Delivery Partner may contact the Customer by telephone, SMS, or other available means to coordinate Delivery.',
              ],
            },
          ],
        },
        {
          id: 'delivery-waiting-time',
          number: '10.6',
          heading: 'Delivery Waiting Time',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Upon arrival at the Delivery Address, Delivery personnel shall make reasonable efforts to contact the Customer and shall wait for up to ten (10) minutes unless otherwise agreed. Where the Customer fails to receive the Order within this period, the Delivery shall be treated as unsuccessful.',
              ],
            },
          ],
        },
        {
          id: 'failed-delivery',
          number: '10.7',
          heading: 'Failed Delivery',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'A Delivery Attempt may fail for reasons including the Customer being unavailable, unreachable, or refusing Delivery; estate or security restrictions preventing access; an incorrect or incomplete address; or other circumstances attributable to the Customer.',
              ],
            },
          ],
        },
        {
          id: 'rescheduling-fee-and-second-delivery',
          number: '10.8',
          heading: 'Rescheduling Fee and Second Delivery',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'If Delivery cannot be completed due to Customer unavailability, an incorrect Delivery Address, or any other circumstance attributable to the Customer, the Customer shall bear the full cost of the subsequent Delivery Attempt. The applicable rescheduling fee shall be equal to the Delivery Fee in effect at the time the new Delivery is scheduled. Where the failed Delivery results from the actions or omissions of the operator, its employees, agents, or Delivery Partners, no rescheduling fee shall apply and the operator shall bear the cost of the subsequent Delivery Attempt.',
              ],
            },
          ],
        },
        {
          id: 'delivery-delays',
          number: '10.9',
          heading: 'Delivery Delays',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Delivery may be delayed by circumstances including heavy rain, flooding, fuel shortages, government restrictions, road closures, civil unrest, security issues, public holidays, and other events of Force Majeure. The operator shall use reasonable efforts to notify the Customer of material delays and to complete Delivery as soon as reasonably practicable.',
              ],
            },
          ],
        },
        {
          id: 'customer-refusal-at-delivery',
          number: '10.10',
          heading: 'Customer Refusal at Delivery',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Where a Customer refuses to accept an Order upon Delivery for reasons not attributable to the operator, the refusal shall be treated as a cancellation after Fulfillment. The operator may, at its sole discretion, waive applicable charges as a gesture of goodwill. Such waiver shall not create any ongoing entitlement for future Orders.',
              ],
            },
          ],
        },
        {
          id: 'proof-of-delivery',
          number: '10.11',
          heading: 'Proof of Delivery',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Delivery may be completed through electronic signature together with a delivery photograph where applicable. Such records may be retained by the operator for delivery verification, dispute resolution, and quality assurance. Electronic signatures and delivery photographs constitute valid evidence of completion of Delivery.',
              ],
            },
          ],
        },
        {
          id: 'delivery-completion',
          number: '10.12',
          heading: 'Delivery Completion',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Delivery is complete when the Products have been handed over to the Customer or authorised recipient at the delivery address, or when Proof of Delivery has been recorded in accordance with these Terms. Risk in the Products passes to the Customer upon completion of Delivery.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'products',
      number: '11',
      heading: 'Products',
      children: [
        {
          id: 'product-quality',
          number: '11.1',
          heading: 'Product Quality',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator shall supply Products that meet commercially reasonable quality standards. Beans shall be clean and free from excessive stones, dirt, weevils, and other foreign materials. Fresh produce shall be reasonably fit for consumption and free from significant spoilage at the time of Delivery. Agricultural commodities such as rice, tomatoes, and vegetables are subject to natural variations in size, colour, texture, and appearance. Such natural variations do not constitute a defect.',
              ],
            },
          ],
        },
        {
          id: 'weights-and-measurements',
          number: '11.2',
          heading: 'Weights and Measurements',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Products shall be sold using the unit specified at the time of purchase. The operator may sell Products by weight, measure, count, or other applicable unit depending on the Product category. Weights and measurements stated on the Platform are approximate and are subject to commercially reasonable tolerances.',
              ],
            },
          ],
        },
        {
          id: 'packaging',
          number: '11.3',
          heading: 'Packaging',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Disposable packaging materials are not returnable. Reusable crates supplied by the operator remain the property of the operator unless purchased by the Customer. Customers shall return reusable crates in good condition or pay the applicable replacement cost.',
              ],
            },
          ],
        },
        {
          id: 'perishable-products',
          number: '11.4',
          heading: 'Perishable Products',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Perishable Products shall be handled and transported with reasonable care. The Customer is responsible for inspecting perishable Products upon Delivery and for storing them appropriately thereafter.',
              ],
            },
          ],
        },
        {
          id: 'commodity-standards',
          number: '11.5',
          heading: 'Commodity Standards',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator sources agricultural commodities in accordance with prevailing market standards in Kaduna and Nigeria. The Customer acknowledges that agricultural products are subject to seasonal availability, weather conditions, and other factors that may affect quality and supply.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'pricing',
      number: '12',
      heading: 'Pricing',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Product prices are subject to change before Order Confirmation due to market conditions, supplier pricing, or procurement costs. Prices displayed on the Platform at the time of Order Submission are indicative. Final prices are verified prior to Order Confirmation.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Where a price change affects a submitted Order, the operator shall notify the Customer before Order Confirmation. The Customer may accept the revised price or cancel the affected portion of the Order without penalty, and a Refund shall be processed in respect of that portion.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Once Order Confirmation has been issued, the confirmed price applies and is not subject to further adjustment except as required by law or as otherwise agreed in writing.',
          ],
        },
      ],
    },

    {
      id: 'promotions',
      number: '13',
      heading: 'Promotions',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator may offer Promotions from time to time, including promo codes, coupons, and referral rewards. Promotional codes are subject to their stated terms and validity period. Expired promotional codes shall not be accepted.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            "Where a promotional code issued by the operator fails due to a technical error, the operator may issue a replacement code or apply an equivalent promotional benefit at its discretion. Referral rewards shall only be granted where eligibility can be verified through the operator's records.",
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Promotions may be subject to usage limits, minimum order values, and other conditions. Abuse of Promotions, including the use of multiple Accounts to obtain promotional benefits, may result in cancellation of the Promotion, suspension of the Account, and recovery of any benefit improperly obtained.',
          ],
        },
      ],
    },

    {
      id: 'returns-refunds-and-complaints',
      number: '14',
      heading: 'Returns, Refunds and Complaints',
      children: [
        {
          id: 'damaged-wrong-or-missing-products',
          number: '14.1',
          heading: 'Damaged, Wrong, or Missing Products',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The Customer shall inspect the Products upon Delivery. Where Products are damaged, wrong, or missing, the Customer shall notify the operator as soon as reasonably practicable and in any event within forty-eight (48) hours of Delivery, providing such details and evidence as the operator may reasonably request.',
              ],
            },
          ],
        },
        {
          id: 'late-deliveries',
          number: '14.2',
          heading: 'Late Deliveries',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Where Delivery exceeds the guaranteed six (6) hour Delivery Window for reasons attributable to the operator and not to Force Majeure or Customer-caused delay, the Customer may raise a complaint. The operator shall consider the complaint in good faith and may offer a partial Refund of the Delivery Fee or other appropriate remedy at its discretion.',
              ],
            },
          ],
        },
        {
          id: 'complaint-handling',
          number: '14.3',
          heading: 'Complaint Handling',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Customer service complaints shall be acknowledged as soon as reasonably practicable during published business hours. Complaints shall be submitted through the official support channels. The operator shall use reasonable efforts to resolve them fairly and promptly.',
              ],
            },
          ],
        },
        {
          id: 'refund-timelines',
          number: '14.4',
          heading: 'Refund Timelines',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Approved Refunds shall ordinarily be processed within seven (7) to fourteen (14) Business Days to the original payment method, subject to the processing times of the relevant payment provider.',
              ],
            },
          ],
        },
        {
          id: 'non-returnable-items',
          number: '14.5',
          heading: 'Non-Returnable Items',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Disposable packaging is not returnable. Perishable Products that have been accepted and are not damaged or defective at the time of Delivery are not eligible for return except as required by applicable law.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'customer-responsibilities',
      number: '15',
      heading: 'Customer Responsibilities',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The Customer shall provide accurate and complete information when registering an Account and placing Orders, including a correct delivery address and a reachable contact telephone number.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The Customer shall ensure availability at the delivery address during the Delivery Window, or shall arrange for an authorised recipient to be present.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Customers shall treat the operator employees and Delivery personnel with courtesy and respect. Abusive, threatening, or discriminatory conduct may result in cancellation of Orders, suspension of Accounts, or refusal of future service.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The Customer shall use the Platform and Services only for lawful purposes and in accordance with these Terms. The Customer shall maintain only one active Account unless otherwise approved by the operator.',
          ],
        },
      ],
    },

    {
      id: 'company-responsibilities',
      number: '16',
      heading: 'Company Responsibilities',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator shall provide the Services with reasonable care and skill. The operator shall use reasonable efforts to supply Products of commercially reasonable quality and to complete Delivery within the applicable Delivery Window.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The operator shall ensure that Delivery Partners receive appropriate training and carry company identification. The operator shall handle complaints in accordance with these Terms and shall use reasonable efforts to resolve them fairly.',
          ],
        },
      ],
    },

    {
      id: 'driver-conduct',
      number: '17',
      heading: 'Driver Conduct',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Delivery personnel shall undergo appropriate training, carry valid company identification, and conduct themselves professionally. Customers may request identification before accepting Delivery.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            "Delivery Partners shall not harass, discriminate against, or engage in any improper conduct toward Customers. Delivery Partners shall not enter the Customer's premises without authorisation. Delivery shall be completed at the designated delivery point or as otherwise agreed with the Customer.",
          ],
        },
      ],
    },

    {
      id: 'fraud-prevention',
      number: '18',
      heading: 'Fraud Prevention',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Customers are permitted to maintain only one active Account unless otherwise approved by the operator. The operator may use technical measures, including IP address monitoring and account activity analysis, to detect fraudulent or abusive conduct. The operator reserves the right to investigate suspicious activity and suspend or terminate affected Accounts.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Where the operator reasonably believes that an Account has been used fraudulently or in breach of these Terms, the operator may suspend or terminate the Account, cancel pending Orders, withhold Refunds, and take such other action as is reasonably necessary, including reporting to law enforcement authorities.',
          ],
        },
      ],
    },

    {
      id: 'business-customers',
      number: '19',
      heading: 'Business Customers',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Business Customers may place Recurring Orders, standing Orders, bulk Orders, and corporate Orders through the Platform or as otherwise arranged with the operator. Each Recurring Order instance generates a new Order ID and a new Payment ID and is treated as a separate Order under these Terms.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Business Customers are responsible for ensuring that authorised personnel place Orders on their behalf and that payment obligations are met. The operator may require additional commercial terms for significant volume or recurring arrangements.',
          ],
        },
      ],
    },

    {
      id: 'communications',
      number: '20',
      heading: 'Communications',
      blocks: [
        {
          type: 'paragraph',
          content: [
            "The operator may communicate with the Customer by electronic means, including email, SMS, WhatsApp, Platform notifications, and other channels. Notices under these Terms shall be effective when sent to the contact details associated with the Customer's Account.",
          ],
        },
        {
          type: 'paragraph',
          content: [
            "The Customer is responsible for maintaining current contact details. Where communication channels are temporarily unavailable, the operator shall use reasonable efforts to notify Customers through alternative available channels. The operator shall not be liable for communication failures arising from circumstances beyond its reasonable control, including network outages or the Customer's failure to maintain reachable contact information.",
          ],
        },
      ],
    },

    {
      id: 'privacy',
      number: '21',
      heading: 'Privacy',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator processes personal data in accordance with its Privacy Policy, which is available on the Platform and is incorporated into these Terms by reference. By using the Platform, the Customer consents to the processing of personal data as described in the Privacy Policy.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Delivery photographs and electronic signatures may be collected as Proof of Delivery. Location and tracking data may be processed in connection with Delivery. The Customer acknowledges that such processing is necessary for the performance of the Services.',
          ],
        },
      ],
    },

    {
      id: 'intellectual-property',
      number: '22',
      heading: 'Intellectual Property',
      blocks: [
        {
          type: 'paragraph',
          content: [
            "All intellectual property rights in the Platform, including the operator's name, logos, branding, content, software, and design, belong to the operator or its licensors. The Customer is granted a limited, non-exclusive, non-transferable licence to access and use the Platform solely for the purpose of placing Orders and using the Services in accordance with these Terms.",
          ],
        },
        {
          type: 'paragraph',
          content: [
            'The Customer shall not copy, modify, distribute, reverse engineer, or create derivative works from any part of the Platform without the prior written consent of the operator.',
          ],
        },
      ],
    },

    {
      id: 'right-to-refuse-service',
      number: '23',
      heading: 'Right to Refuse Service',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'The operator reserves the right to refuse, suspend, or cancel Orders where reasonably necessary, including cases involving suspected fraud, abusive conduct, repeated failed Deliveries, payment irregularities, safety concerns, or legal and regulatory requirements.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Where the operator exercises this right after payment has been received, the operator shall process a Refund of amounts paid in respect of Products not delivered, subject to any deductions permitted under these Terms.',
          ],
        },
      ],
    },

    {
      id: 'limitation-of-liability',
      number: '24',
      heading: 'Limitation of Liability',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'Nothing in these Terms excludes or limits liability for death or personal injury caused by negligence, fraud or fraudulent misrepresentation, or any other liability that cannot be excluded or limited under applicable Nigerian law.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Subject to the preceding paragraph, the operator shall not be liable for any indirect, incidental, special, consequential, or punitive damages, including loss of profit, loss of business, loss of data, or loss of goodwill, arising out of or in connection with the use of the Platform or the Services, whether in contract, tort (including negligence), or otherwise.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            "Subject to the first paragraph of this Section, the operator's total aggregate liability to the Customer in respect of any Order shall not exceed the total amount paid by the Customer for that Order.",
          ],
        },
        {
          type: 'paragraph',
          content: [
            'These limitations apply to the fullest extent permitted by applicable consumer protection and other laws. Where mandatory consumer protection provisions apply and cannot be limited, those provisions shall prevail to the extent of any inconsistency.',
          ],
        },
      ],
    },

    {
      id: 'force-majeure',
      number: '25',
      heading: 'Force Majeure',
      blocks: [
        {
          type: 'paragraph',
          content: [
            "The operator shall not be liable for any failure or delay in performing its obligations under these Terms where such failure or delay results from Force Majeure. Events of Force Majeure include natural disasters, flooding, heavy rain, war, riots, civil unrest, government action or restriction, road closures, fuel shortages, security incidents, public holidays, network or system outages, and any other cause beyond the operator's reasonable control.",
          ],
        },
        {
          type: 'paragraph',
          content: [
            'Where a Force Majeure event continues for more than fourteen (14) days, either party may cancel the affected Order without liability, and the operator shall process a Refund of amounts paid in respect of Products not delivered.',
          ],
        },
      ],
    },

    {
      id: 'dispute-resolution',
      number: '26',
      heading: 'Dispute Resolution',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'In the event of any dispute arising out of or in connection with these Terms or an Order, the parties shall first attempt to resolve the dispute in good faith through negotiation. The Customer shall contact the operator through the official support channels and allow a reasonable period for resolution.',
          ],
        },
        {
          type: 'paragraph',
          content: [
            'These Terms and any dispute arising under them shall be governed by and construed in accordance with the laws of the Federal Republic of Nigeria. Subject to any mandatory consumer protection provisions, the courts of Kaduna State, Nigeria, shall have exclusive jurisdiction to settle any dispute arising out of or in connection with these Terms.',
          ],
        },
      ],
    },

    {
      id: 'general-provisions',
      number: '27',
      heading: 'General Provisions',
      children: [
        {
          id: 'severability',
          number: '27.1',
          heading: 'Severability',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'If any provision of these Terms is held to be invalid, illegal, or unenforceable by a court of competent jurisdiction, that provision shall be modified to the minimum extent necessary to make it valid and enforceable, or if modification is not possible, shall be severed from these Terms. The remaining provisions shall continue in full force and effect.',
              ],
            },
          ],
        },
        {
          id: 'entire-agreement',
          number: '27.2',
          heading: 'Entire Agreement',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'These Terms, together with the Privacy Policy and any Order Confirmation, constitute the entire agreement between the Customer and the operator in respect of the subject matter and supersede all prior discussions, representations, and agreements relating thereto.',
              ],
            },
          ],
        },
        {
          id: 'assignment',
          number: '27.3',
          heading: 'Assignment',
          blocks: [
            {
              type: 'paragraph',
              content: [
                "The Customer may not assign or transfer any rights or obligations under these Terms without the prior written consent of the operator. The operator may assign or transfer its rights and obligations under these Terms to any affiliate or successor without the Customer's consent.",
              ],
            },
          ],
        },
        {
          id: 'waiver',
          number: '27.4',
          heading: 'Waiver',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'No failure or delay by the operator in exercising any right under these Terms shall constitute a waiver of that right. Any waiver must be in writing and signed by an authorised representative of the operator to be effective.',
              ],
            },
          ],
        },
        {
          id: 'amendments',
          number: '27.5',
          heading: 'Amendments',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'The operator may amend these Terms from time to time. The amended Terms shall be effective upon publication on the Platform or such later date as specified. Material changes shall be notified to Customers by reasonable means. Continued use of the Platform after the effective date constitutes acceptance of the amended Terms.',
              ],
            },
          ],
        },
        {
          id: 'electronic-records',
          number: '27.6',
          heading: 'Electronic Records',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Electronic records, including Order Confirmations, electronic signatures, delivery photographs, and Platform communications, shall be admissible as evidence and shall have the same legal effect as paper records to the fullest extent permitted by applicable law.',
              ],
            },
          ],
        },
        {
          id: 'survival',
          number: '27.7',
          heading: 'Survival',
          blocks: [
            {
              type: 'paragraph',
              content: [
                'Provisions of these Terms that by their nature are intended to survive termination or expiry, including those relating to intellectual property, limitation of liability, dispute resolution, and general provisions, shall survive.',
              ],
            },
          ],
        },
      ],
    },

    {
      id: 'contact-information',
      number: '28',
      heading: 'Contact Information',
      blocks: [
        {
          type: 'paragraph',
          content: [
            'For enquiries, complaints, or support relating to these Terms, Orders, or the Services, the Customer may contact the operator through the following official channels:',
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
            {
              label: 'Website',
              value: 'www.example.com',
              href: 'https://example.com',
            },
            { label: 'Business address', value: 'Kaduna, Nigeria' },
          ],
        },
      ],
    },
  ],

  closing: 'These Terms and Conditions are effective as of 5 August 2026.',
};
